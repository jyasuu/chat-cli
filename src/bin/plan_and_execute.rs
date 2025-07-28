use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use schemars::{JsonSchema, schema_for};
use std::env;
use std::io::Write;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use chat_cli::openai::OpenAIClient;
use chat_cli::function_calling::{FunctionExecutor, FunctionCall};
use chat_cli::chat_client::ChatClient;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Plan {
    /// Different steps to follow, should be in sorted order
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_number: usize,
    pub step_description: String,
    pub result: String,
    pub success: bool,
}

pub struct PlanAndExecuteAgent {
    planner_client: OpenAIClient,
    executor_client: OpenAIClient,
    function_executor: FunctionExecutor,
}

impl PlanAndExecuteAgent {
    pub fn new(api_key: String, model: String) -> Self {
        let mut planner_client = OpenAIClient::new(api_key.clone(), model.clone());
        let mut executor_client = OpenAIClient::new(api_key, model);
        
        // Set up the planner with structured output for plan generation
        let plan_schema = schema_for!(Plan);
        let plan_schema_json = serde_json::to_value(plan_schema).unwrap();
        planner_client.set_structured_output("Plan", plan_schema_json);
        
        // Load system prompts
        let planner_prompt = "You are a strategic planning assistant. For any given objective, create a simple step-by-step plan that will lead to the correct answer. Each step should be specific, actionable, and contain all necessary information. Do not add superfluous steps. The final step should yield the complete answer to the objective.";
        let executor_prompt = "You are a helpful assistant that executes individual steps of a plan. You have access to various tools to help you complete tasks. Use the available tools when needed to gather information, perform calculations, or complete actions. Be thorough and accurate in your execution.";
        
        planner_client.load_system_prompt(planner_prompt).unwrap();
        executor_client.load_system_prompt(executor_prompt).unwrap();
        
        // Set up function executor with available tools
        let function_executor = FunctionExecutor::new();
        
        // Provide tools to the executor client
        executor_client.set_available_tools(function_executor.get_available_tools());
        
        Self {
            planner_client,
            executor_client,
            function_executor,
        }
    }
    
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.planner_client = self.planner_client.with_base_url(base_url.clone());
        self.executor_client = self.executor_client.with_base_url(base_url);
        self
    }
    
    /// Generate a plan for the given objective
    pub async fn create_plan(&mut self, objective: &str) -> Result<Plan> {
        println!("🎯 Creating plan for objective: {}", objective);
        
        let planning_prompt = format!(
            "For the given objective, come up with a simple step by step plan. This plan should involve individual tasks, that if executed correctly will yield the correct answer. Do not add any superfluous steps. The result of the final step should be the final answer. Make sure that each step has all the information needed - do not skip steps.\n\nYour objective: {}",
            objective
        );
        
        self.planner_client.clear_conversation();
        self.planner_client.add_user_message(&planning_prompt);
        
        let response = self.planner_client.send_message().await?;
        
        // Parse the structured response
        let plan: Plan = serde_json::from_str(&response)
            .map_err(|e| anyhow!("Failed to parse plan from response: {}. Response was: {}", e, response))?;
        
        println!("📋 Generated plan with {} steps:", plan.steps.len());
        for (i, step) in plan.steps.iter().enumerate() {
            println!("   {}. {}", i + 1, step);
        }
        
        Ok(plan)
    }
    
    /// Execute a single step of the plan
    pub async fn execute_step(&mut self, step_number: usize, step_description: &str, plan_context: &[String]) -> Result<StepResult> {
        println!("\n🔧 Executing step {}: {}", step_number, step_description);
        
        // Create context for the step execution
        let plan_context_str = plan_context.iter()
            .enumerate()
            .map(|(i, step)| format!("{}. {}", i + 1, step))
            .collect::<Vec<_>>()
            .join("\n");
        
        let execution_prompt = format!(
            "For the following plan:\n{}\n\nYou are tasked with executing step {}: {}.\n\nUse the available tools if needed to complete this step. Provide a clear and complete result.",
            plan_context_str,
            step_number,
            step_description
        );
        
        // Clear conversation and set up for this step
        self.executor_client.clear_conversation();
        self.executor_client.add_user_message(&execution_prompt);
        
        // Execute with streaming to handle function calls
        let mut receiver = self.executor_client.send_message_stream().await?;
        let mut response_text = String::new();
        let mut pending_function_calls = Vec::new();
        
        // Process streaming response
        while let Some((text_chunk, function_call)) = receiver.recv().await {
            if !text_chunk.is_empty() {
                print!("{}", text_chunk);
                response_text.push_str(&text_chunk);
            }
            
            if let Some(fc) = function_call {
                pending_function_calls.push(fc);
            }
        }
        
        // Execute any function calls
        for fc_json in pending_function_calls {
            if let Ok(function_call) = serde_json::from_value::<FunctionCall>(fc_json) {
                println!("\n🔧 Executing function: {}", function_call.name);
                
                match self.function_executor.execute_function(&function_call).await {
                    Ok(function_response) => {
                        // Add function response to conversation
                        self.executor_client.add_model_response(&response_text, vec![serde_json::json!({
                            "name": function_call.name,
                            "args": function_call.args
                        })]);
                        self.executor_client.add_function_response(&function_response);
                        
                        // Get follow-up response
                        let follow_up_response = self.executor_client.send_message().await?;
                        response_text.push_str("\n");
                        response_text.push_str(&follow_up_response);
                        print!("{}", follow_up_response);
                    }
                    Err(e) => {
                        let error_msg = format!("Function execution failed: {}", e);
                        println!("\n❌ {}", error_msg);
                        response_text.push_str(&format!("\nError: {}", error_msg));
                    }
                }
            }
        }
        
        println!(); // New line after step completion
        
        Ok(StepResult {
            step_number,
            step_description: step_description.to_string(),
            result: response_text,
            success: true,
        })
    }
    
    /// Execute the complete plan
    pub async fn execute_plan(&mut self, plan: &Plan) -> Result<Vec<StepResult>> {
        let mut results = Vec::new();
        
        println!("\n🚀 Starting plan execution...\n");
        
        for (i, step) in plan.steps.iter().enumerate() {
            let step_number = i + 1;
            
            match self.execute_step(step_number, step, &plan.steps).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    println!("❌ Step {} failed: {}", step_number, e);
                    results.push(StepResult {
                        step_number,
                        step_description: step.clone(),
                        result: format!("Failed: {}", e),
                        success: false,
                    });
                    // Continue with remaining steps even if one fails
                }
            }
        }
        
        Ok(results)
    }
    
    /// Run the complete plan and execute workflow
    pub async fn plan_and_execute(&mut self, objective: &str) -> Result<String> {
        // Step 1: Create the plan
        let plan = self.create_plan(objective).await?;
        
        // Step 2: Execute the plan
        let results = self.execute_plan(&plan).await?;
        
        // Step 3: Summarize results
        println!("\n📊 Plan Execution Summary:");
        println!("{}", "=".repeat(50));
        
        let mut final_answer = String::new();
        
        for result in &results {
            let status = if result.success { "✅" } else { "❌" };
            println!("{} Step {}: {}", status, result.step_number, result.step_description);
            
            if result.success {
                final_answer = result.result.clone(); // Use the last successful result as final answer
            }
        }
        
        println!("\n🎯 Final Answer:");
        println!("{}", final_answer);
        
        Ok(final_answer)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();
    
    let api_key = env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow!("OPENAI_API_KEY environment variable not set"))?;
    
    let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4".to_string());
    let base_url = env::var("OPENAI_BASE_URL").ok();
    
    // Create the agent
    let mut agent = if let Some(url) = base_url {
        PlanAndExecuteAgent::new(api_key, model).with_base_url(url)
    } else {
        PlanAndExecuteAgent::new(api_key, model)
    };
    
    println!("🤖 Plan and Execute Agent");
    println!("Type your objective and I'll create a plan and execute it step by step.");
    println!("Type 'quit' or 'exit' to stop.\n");
    
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    
    loop {
        print!("📝 Enter your objective: ");
        std::io::stdout().flush().unwrap();
        
        line.clear();
        reader.read_line(&mut line).await?;
        let objective = line.trim();
        
        if objective.is_empty() {
            continue;
        }
        
        if objective.eq_ignore_ascii_case("quit") || objective.eq_ignore_ascii_case("exit") {
            println!("👋 Goodbye!");
            break;
        }
        
        println!("\n{}", "=".repeat(60));
        
        match agent.plan_and_execute(objective).await {
            Ok(_) => {
                println!("\n✅ Objective completed successfully!");
            }
            Err(e) => {
                println!("\n❌ Failed to complete objective: {}", e);
            }
        }
        
        println!("\n{}\n", "=".repeat(60));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_plan_creation() {
        // This test requires API key to be set
        if env::var("OPENAI_API_KEY").is_err() {
            return;
        }
        
        let api_key = env::var("OPENAI_API_KEY").unwrap();
        let mut agent = PlanAndExecuteAgent::new(api_key, "gpt-3.5-turbo".to_string());
        
        let objective = "What is 2 + 2?";
        let plan = agent.create_plan(objective).await.unwrap();
        
        assert!(!plan.steps.is_empty());
        assert!(plan.steps.len() <= 5); // Should be a simple plan
    }
}