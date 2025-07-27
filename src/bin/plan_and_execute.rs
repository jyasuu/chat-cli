use std::collections::HashMap;
use std::fmt;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio;
use std::env;

use chat_cli::chat_client::{AnyChatClient, ChatClient};

// Core types and traits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub data: HashMap<String, serde_json::Value>,
}

impl State {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.data.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn set<T>(&mut self, key: &str, value: T)
    where
        T: Serialize,
    {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.data.insert(key.to_string(), json_value);
        }
    }

    pub fn merge(&mut self, other: State) {
        for (key, value) in other.data {
            self.data.insert(key, value);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub action: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub completed: bool,
}

#[derive(Debug)]
pub enum ExecutionError {
    NodeNotFound(String),
    ExecutionFailed(String),
    PlanningFailed(String),
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExecutionError::NodeNotFound(name) => write!(f, "Node not found: {}", name),
            ExecutionError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            ExecutionError::PlanningFailed(msg) => write!(f, "Planning failed: {}", msg),
        }
    }
}

impl std::error::Error for ExecutionError {}

#[async_trait]
pub trait Node: Send + Sync {
    async fn execute(&self, state: &mut State) -> Result<State, ExecutionError>;
    fn name(&self) -> &str;
}

// Planning node
pub struct PlannerNode {
    name: String,
    client: AnyChatClient,
}

impl PlannerNode {
    pub fn new(name: &str) -> Self {
        let gemini_api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
        let client = AnyChatClient::new_gemini(gemini_api_key, "gemini-1.5-flash-latest".to_string());
        Self {
            name: name.to_string(),
            client,
        }
    }

    async fn create_plan(&mut self, objective: &str) -> Result<Plan, ExecutionError> {
        let prompt = format!(
            r#"
Create a plan to achieve the following objective: "{}"

The plan should be a JSON object with a "steps" array. Each step should have the following fields:
- "id": A unique identifier for the step (e.g., "step1").
- "action": A short, actionable verb phrase (e.g., "gather_sources", "write_draft").
- "description": A detailed description of what the step entails.
- "dependencies": A list of step IDs that must be completed before this step can start.
- "completed": Should be initialized to false.

Example:
{{
  "steps": [
    {{
      "id": "step1",
      "action": "analyze_requirements",
      "description": "Understand the requirements of the objective.",
      "dependencies": [],
      "completed": false
    }},
    {{
      "id": "step2",
      "action": "execute_task",
      "description": "Perform the main task based on the requirements.",
      "dependencies": ["step1"],
      "completed": false
    }}
  ]
}}

Now, generate the plan for the objective: "{}"
"#,
            objective, objective
        );

        self.client.add_user_message(&prompt);
        let response = self.client.send_message().await
            .map_err(|e| ExecutionError::PlanningFailed(e.to_string()))?;

        // Clean the response to extract only the JSON part
        let json_response = response
            .trim()
            .replace("```json", "")
            .replace("```", "")
            .trim()
            .to_string();

        serde_json::from_str::<Plan>(&json_response)
            .map_err(|e| ExecutionError::PlanningFailed(format!("Failed to parse plan: {}. Response: {}", e, json_response)))
    }
}

#[async_trait]
impl Node for PlannerNode {
    async fn execute(&self, state: &mut State) -> Result<State, ExecutionError> {
        let objective: String = state.get("objective")
            .ok_or_else(|| ExecutionError::PlanningFailed("No objective found in state".to_string()))?;

        // We need to make self mutable, but since this is a trait method, we need to work around it
        // For now, we'll create a new client instance
        let gemini_api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
        let mut temp_client = AnyChatClient::new_gemini(gemini_api_key, "gemini-1.5-flash-latest".to_string());
        
        let prompt = format!(
            r#"
Create a plan to achieve the following objective: "{}"

The plan should be a JSON object with a "steps" array. Each step should have the following fields:
- "id": A unique identifier for the step (e.g., "step1").
- "action": A short, actionable verb phrase (e.g., "gather_sources", "write_draft").
- "description": A detailed description of what the step entails.
- "dependencies": A list of step IDs that must be completed before this step can start.
- "completed": Should be initialized to false.

Example:
{{
  "steps": [
    {{
      "id": "step1",
      "action": "analyze_requirements",
      "description": "Understand the requirements of the objective.",
      "dependencies": [],
      "completed": false
    }},
    {{
      "id": "step2",
      "action": "execute_task",
      "description": "Perform the main task based on the requirements.",
      "dependencies": ["step1"],
      "completed": false
    }}
  ]
}}

Now, generate the plan for the objective: "{}"
"#,
            objective, objective
        );

        temp_client.add_user_message(&prompt);
        let response = temp_client.send_message().await
            .map_err(|e| ExecutionError::PlanningFailed(e.to_string()))?;

        // Clean the response to extract only the JSON part
        let json_response = response
            .trim()
            .replace("```json", "")
            .replace("```", "")
            .trim()
            .to_string();

        let plan = serde_json::from_str::<Plan>(&json_response)
            .map_err(|e| ExecutionError::PlanningFailed(format!("Failed to parse plan: {}. Response: {}", e, json_response)))?;
        
        let mut new_state = State::new();
        new_state.set("plan", &plan);
        new_state.set("current_step", 0usize);
        
        println!("📋 Plan created with {} steps", plan.steps.len());
        for (i, step) in plan.steps.iter().enumerate() {
            println!("  {}. {} - {}", i + 1, step.action, step.description);
        }
        
        Ok(new_state)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// Execution node
pub struct ExecutorNode {
    name: String,
    action_handlers: HashMap<String, Box<dyn ActionHandler>>,
}

#[async_trait]
pub trait ActionHandler: Send + Sync {
    async fn handle(&self, description: &str, state: &State) -> Result<String, ExecutionError>;
}

// Mock action handlers
pub struct ResearchHandler;

#[async_trait]
impl ActionHandler for ResearchHandler {
    async fn handle(&self, description: &str, _state: &State) -> Result<String, ExecutionError> {
        println!("  -> Researching: {}", description);
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        Ok(format!("Completed: {}", description))
    }
}

pub struct WriteHandler;

#[async_trait]
impl ActionHandler for WriteHandler {
    async fn handle(&self, description: &str, _state: &State) -> Result<String, ExecutionError> {
        println!("  -> Writing: {}", description);
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        Ok(format!("Written: {}", description))
    }
}

pub struct DefaultHandler;

#[async_trait]
impl ActionHandler for DefaultHandler {
    async fn handle(&self, description: &str, _state: &State) -> Result<String, ExecutionError> {
        println!("  -> Executing: {}", description);
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        Ok(format!("Executed: {}", description))
    }
}

impl ExecutorNode {
    pub fn new(name: &str) -> Self {
        let mut action_handlers: HashMap<String, Box<dyn ActionHandler>> = HashMap::new();
        // Generic handlers
        action_handlers.insert("gather_sources".to_string(), Box::new(ResearchHandler));
        action_handlers.insert("analyze_information".to_string(), Box::new(ResearchHandler));
        action_handlers.insert("synthesize_results".to_string(), Box::new(ResearchHandler));
        action_handlers.insert("research".to_string(), Box::new(ResearchHandler));
        action_handlers.insert("outline".to_string(), Box::new(WriteHandler));
        action_handlers.insert("draft".to_string(), Box::new(WriteHandler));
        action_handlers.insert("write".to_string(), Box::new(WriteHandler));
        action_handlers.insert("revise".to_string(), Box::new(WriteHandler));
        action_handlers.insert("default".to_string(), Box::new(DefaultHandler));

        Self {
            name: name.to_string(),
            action_handlers,
        }
    }

    fn get_next_executable_step(&self, plan: &Plan) -> Option<usize> {
        for (i, step) in plan.steps.iter().enumerate() {
            if !step.completed {
                let deps_completed = step.dependencies.iter().all(|dep_id| {
                    plan.steps.iter().any(|s| s.id == *dep_id && s.completed)
                });
                
                if deps_completed {
                    return Some(i);
                }
            }
        }
        None
    }
}

#[async_trait]
impl Node for ExecutorNode {
    async fn execute(&self, state: &mut State) -> Result<State, ExecutionError> {
        let mut plan: Plan = state.get("plan")
            .ok_or_else(|| ExecutionError::ExecutionFailed("No plan found in state".to_string()))?;

        if let Some(step_idx) = self.get_next_executable_step(&plan) {
            let step = &mut plan.steps[step_idx];
            
            println!("🔄 Executing step: {} - {}", step.action, step.description);
            
            let handler = self.action_handlers.get(&step.action)
                .or_else(|| self.action_handlers.get("default"))
                .ok_or_else(|| ExecutionError::ExecutionFailed(
                    format!("No handler found for action: {}", step.action)
                ))?;

            let result = handler.handle(&step.description, state).await?;
            step.completed = true;
            
            println!("✅ {}", result);
            
            let mut new_state = State::new();
            new_state.set("plan", &plan);
            new_state.set("last_result", result);
            
            let all_completed = plan.steps.iter().all(|s| s.completed);
            new_state.set("execution_complete", all_completed);
            
            Ok(new_state)
        } else {
            let mut new_state = State::new();
            new_state.set("plan", &plan);
            new_state.set("execution_complete", true);
            Ok(new_state)
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// Main workflow orchestrator
pub struct PlanAndExecuteWorkflow {
    planner: PlannerNode,
    executor: ExecutorNode,
}

impl PlanAndExecuteWorkflow {
    pub fn new() -> Self {
        Self {
            planner: PlannerNode::new("planner"),
            executor: ExecutorNode::new("executor"),
        }
    }

    pub async fn run(&self, objective: &str) -> Result<State, ExecutionError> {
        println!("🚀 Starting plan-and-execute workflow");
        println!("📝 Objective: {}", objective);
        
        let mut state = State::new();
        state.set("objective", objective.to_string());

        println!("\n📋 PLANNING PHASE");
        let plan_result = self.planner.execute(&mut state).await?;
        state.merge(plan_result);

        println!("\n⚡ EXECUTION PHASE");
        loop {
            let exec_result = self.executor.execute(&mut state).await?;
            state.merge(exec_result);
            
            let execution_complete: bool = state.get("execution_complete").unwrap_or(false);
            if execution_complete {
                break;
            }
        }

        println!("\n🎉 Workflow completed successfully!");
        Ok(state)
    }
}

// Example usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenv::dotenv().ok();

    let workflow = PlanAndExecuteWorkflow::new();
    
    println!("======================================");
    let result1 = workflow.run("Research the latest trends in AI").await?;
    println!("\nFinal state keys: {:?}", result1.data.keys().collect::<Vec<_>>());
    
    println!("\n{}", "======================================");
    let result2 = workflow.run("Write a blog post about the Rust programming language").await?;
    println!("\nFinal state keys: {:?}", result2.data.keys().collect::<Vec<_>>());
    
    Ok(())
}
