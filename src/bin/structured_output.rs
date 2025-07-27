use anyhow::Result;
use serde::{Deserialize, Serialize};
use schemars::{JsonSchema, schema_for};
use std::env;
use chat_cli::openai::OpenAIClient;
use chat_cli::gemini::GeminiClient;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Response,
    Plan,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Plan {
    /// Different steps to follow, should be in sorted order
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Response {
    /// Response to user
    pub response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action_type", content = "action")]
pub enum Action {
    #[serde(rename = "response")]
    Response(Response),
    #[serde(rename = "plan")]
    Plan(Plan),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "Act")]
pub struct Act {
    /// Action to perform. If you want to respond to user, use Response. If you need to further use tools to get the answer, use Plan.
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimplePlan {
    /// Different steps to follow, should be in sorted order
    pub steps: Vec<String>,
}

async fn test_openai_structured_output() -> Result<()> {
    println!("=== Testing OpenAI Structured Output ===");
    
    let api_key = env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable not set"))?;
    
    let mut client = OpenAIClient::new(api_key, "deepseek/deepseek-chat-v3-0324:free".to_string()).with_base_url("https://openrouter.ai/api/v1".to_string());
    
    // Generate schema
    let schema = schema_for!(Act);
    let schema_json = serde_json::to_value(schema)?;
    
    println!("Generated schema:");
    println!("{}", serde_json::to_string_pretty(&schema_json)?);
    
    // Set structured output
    client.set_structured_output("Act", schema_json);
    
    let prompt = "For the given objective, come up with a simple step by step plan. This plan should involve individual tasks, that if executed correctly will yield the correct answer. Do not add any superfluous steps. The result of the final step should be the final answer. Make sure that each step has all the information needed - do not skip steps.\n\nYour objective was this:\nwhat is the hometown of the mens 2024 Australia open winner?";
    
    client.add_user_message(prompt);
    match client.send_message().await {
        Ok(response) => {
            println!("OpenAI Response:");
            println!("{}", response);
            
            // Try to parse as structured output
            match serde_json::from_str::<Act>(&response) {
                Ok(parsed) => {
                    println!("Successfully parsed structured output:");
                    println!("{:#?}", parsed);
                }
                Err(e) => {
                    println!("Failed to parse as structured output: {}", e);
                }
            }
        }
        Err(e) => {
            println!("OpenAI Error: {}", e);
        }
    }
    
    Ok(())
}

async fn test_gemini_structured_output() -> Result<()> {
    println!("\n=== Testing Gemini Structured Output ===");
    
    let api_key = env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow::anyhow!("GEMINI_API_KEY environment variable not set"))?;
    
    let mut client = GeminiClient::new(api_key, "gemini-1.5-flash".to_string());
    
    // Generate schema for simpler structure
    let schema = schema_for!(SimplePlan);
    let schema_json = serde_json::to_value(schema)?;
    
    println!("Generated schema:");
    println!("{}", serde_json::to_string_pretty(&schema_json)?);
    
    // Set structured output
    client.set_structured_output(schema_json);
    
    let prompt = "For the given objective, come up with a simple step by step plan. This plan should involve individual tasks, that if executed correctly will yield the correct answer. Do not add any superfluous steps. The result of the final step should be the final answer. Make sure that each step has all the information needed - do not skip steps.\n\nYour objective was this:\nwhat is the hometown of the mens 2024 Australia open winner?\n\nRespond with a JSON object containing a 'steps' array.";
    
    client.add_user_message(prompt);
    match client.send_message().await {
        Ok(response) => {
            println!("Gemini Response:");
            println!("{}", response);
            
            // Try to parse as structured output
            match serde_json::from_str::<SimplePlan>(&response) {
                Ok(parsed) => {
                    println!("Successfully parsed structured output:");
                    println!("{:#?}", parsed);
                }
                Err(e) => {
                    println!("Failed to parse as structured output: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Gemini Error: {}", e);
        }
    }
    
    Ok(())
}

async fn test_simple_schema() -> Result<()> {
    println!("\n=== Testing Simple Schema ===");
    
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct SimpleResponse {
        /// The answer to the question
        pub answer: String,
        /// Confidence level from 1-10
        pub confidence: u8,
    }
    
    let schema = schema_for!(SimpleResponse);
    let schema_json = serde_json::to_value(schema)?;
    
    println!("Simple schema:");
    println!("{}", serde_json::to_string_pretty(&schema_json)?);
    
    // Test with OpenAI if available
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        println!("\nTesting simple schema with OpenAI:");
        let mut client = OpenAIClient::new(api_key, "deepseek/deepseek-chat-v3-0324:free".to_string()).with_base_url("https://openrouter.ai/api/v1".to_string());
        client.set_structured_output("SimpleResponse", schema_json.clone());
        
        client.add_user_message("What is 2+2? Provide your confidence level.");
        match client.send_message().await {
            Ok(response) => {
                println!("Response: {}", response);
                match serde_json::from_str::<SimpleResponse>(&response) {
                    Ok(parsed) => println!("Parsed: {:#?}", parsed),
                    Err(e) => println!("Parse error: {}", e),
                }
            }
            Err(e) => println!("Error: {}", e),
        }
    }
    
    // Test with Gemini if available
    if let Ok(api_key) = env::var("GEMINI_API_KEY") {
        println!("\nTesting simple schema with Gemini:");
        let mut client = GeminiClient::new(api_key, "gemini-1.5-flash".to_string());
        client.set_structured_output(schema_json);
        
        client.add_user_message("What is 2+2? Provide your confidence level.");
        match client.send_message().await {
            Ok(response) => {
                println!("Response: {}", response);
                match serde_json::from_str::<SimpleResponse>(&response) {
                    Ok(parsed) => println!("Parsed: {:#?}", parsed),
                    Err(e) => println!("Parse error: {}", e),
                }
            }
            Err(e) => println!("Error: {}", e),
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    
    println!("Structured Output Demo");
    println!("=====================");
    
    // Test simple schema first
    // if let Err(e) = test_simple_schema().await {
    //     println!("Simple schema test failed: {}", e);
    // }
    
    // Test OpenAI structured output
    if let Err(e) = test_openai_structured_output().await {
        println!("OpenAI test failed: {}", e);
    }
    
    // Test Gemini structured output  
    // if let Err(e) = test_gemini_structured_output().await {
    //     println!("Gemini test failed: {}", e);
    // }
    
    println!("\nDemo completed!");
    Ok(())
}




// Structured Output Demo
// =====================

// === Testing Simple Schema ===
// Simple schema:
// {
//   "$schema": "https://json-schema.org/draft/2020-12/schema",
//   "properties": {
//     "answer": {
//       "description": "The answer to the question",
//       "type": "string"
//     },
//     "confidence": {
//       "description": "Confidence level from 1-10",
//       "format": "uint8",
//       "maximum": 255,
//       "minimum": 0,
//       "type": "integer"
//     }
//   },
//   "required": [
//     "answer",
//     "confidence"
//   ],
//   "title": "SimpleResponse",
//   "type": "object"
// }

// Testing simple schema with OpenAI:
// Response: {  
//   "answer": "4",  
//   "confidence": 1  
// }
// Parsed: SimpleResponse {
//     answer: "4",
//     confidence: 1,
// }

// Testing simple schema with Gemini:
// Response: {"answer": "4", "confidence": 10}
// Parsed: SimpleResponse {
//     answer: "4",
//     confidence: 10,
// }
// === Testing OpenAI Structured Output ===
// Generated schema:
// {
//   "$defs": {
//     "Action": {
//       "oneOf": [
//         {
//           "properties": {
//             "action": {
//               "$ref": "#/$defs/Response"
//             },
//             "action_type": {
//               "const": "response",
//               "type": "string"
//             }
//           },
//           "required": [
//             "action_type",
//             "action"
//           ],
//           "type": "object"
//         },
//         {
//           "properties": {
//             "action": {
//               "$ref": "#/$defs/Plan"
//             },
//             "action_type": {
//               "const": "plan",
//               "type": "string"
//             }
//           },
//           "required": [
//             "action_type",
//             "action"
//           ],
//           "type": "object"
//         }
//       ]
//     },
//     "Plan": {
//       "properties": {
//         "steps": {
//           "description": "Different steps to follow, should be in sorted order",
//           "items": {
//             "type": "string"
//           },
//           "type": "array"
//         }
//       },
//       "required": [
//         "steps"
//       ],
//       "type": "object"
//     },
//     "Response": {
//       "properties": {
//         "response": {
//           "description": "Response to user",
//           "type": "string"
//         }
//       },
//       "required": [
//         "response"
//       ],
//       "type": "object"
//     }
//   },
//   "$schema": "https://json-schema.org/draft/2020-12/schema",
//   "properties": {
//     "action": {
//       "$ref": "#/$defs/Action",
//       "description": "Action to perform. If you want to respond to user, use Response. If you need to further use tools to get the answer, use Plan."
//     }
//   },
//   "required": [
//     "action"
//   ],
//   "title": "Act",
//   "type": "object"
// }
// OpenAI Response:
// { "action": { "action": { "steps": [  
//         "Step 1: Identify the winner of the men's 2024 Australian Open.",  
//         "Step 2: Find the hometown of the identified winner.",  
//         "Step 3: Provide the hometown as the final answer."  
//     ] }  
//     , "action_type": "plan"  
// }  

// }
// Successfully parsed structured output:
// Act {
//     action: Plan(
//         Plan {
//             steps: [
//                 "Step 1: Identify the winner of the men's 2024 Australian Open.",
//                 "Step 2: Find the hometown of the identified winner.",
//                 "Step 3: Provide the hometown as the final answer.",
//             ],
//         },
//     ),
// }
// === Testing Gemini Structured Output ===
// Generated schema:
// {
//   "$schema": "https://json-schema.org/draft/2020-12/schema",
//   "properties": {
//     "steps": {
//       "description": "Different steps to follow, should be in sorted order",
//       "items": {
//         "type": "string"
//       },
//       "type": "array"
//     }
//   },
//   "required": [
//     "steps"
//   ],
//   "title": "SimplePlan",
//   "type": "object"
// }
// Gemini Response:
// {"steps": ["Identify the winner of the men's 2024 Australian Open.", "Find the biographical information of the identified winner.", "Locate the hometown information within the biography."]}
// Successfully parsed structured output:
// SimplePlan {
//     steps: [
//         "Identify the winner of the men's 2024 Australian Open.",
//         "Find the biographical information of the identified winner.",
//         "Locate the hometown information within the biography.",
//     ],
// }

// Demo completed!