use serde::{Deserialize, Serialize};
use schemars::{JsonSchema, schema_for};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Address {
    pub street: String,
    pub city: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Contact {
    Address(Address),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "User Schema")]
pub struct UserProfile {
    /// User's unique name
    pub username: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0, max = 120))]
    pub age: Option<u8>,
    
    #[schemars(length(min = 1))]
    pub roles: HashSet<UserRole>,
    
    pub contact: Contact,
}

fn main() {
    // Generate and print the JSON Schema
    let schema = schema_for!(UserProfile);
    let json_schema = serde_json::to_string_pretty(&schema).unwrap();
    println!("{}", json_schema);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_profile_creation() {
        let mut roles = HashSet::new();
        roles.insert(UserRole::Admin);
        
        let address = Address {
            street: "123 Main St".to_string(),
            city: "Anytown".to_string(),
        };
        
        let user = UserProfile {
            username: "john_doe".to_string(),
            age: Some(30),
            roles,
            contact: Contact::Address(address),
        };
        
        // Test serialization
        let json = serde_json::to_string_pretty(&user).unwrap();
        println!("Serialized user: {}", json);
        
        // Test deserialization
        let deserialized: UserProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(user.username, deserialized.username);
    }
    
    #[test]
    fn test_string_contact() {
        let mut roles = HashSet::new();
        roles.insert(UserRole::Viewer);
        
        let user = UserProfile {
            username: "jane_doe".to_string(),
            age: None,
            roles,
            contact: Contact::String("jane@example.com".to_string()),
        };
        
        let json = serde_json::to_string_pretty(&user).unwrap();
        let deserialized: UserProfile = serde_json::from_str(&json).unwrap();
        
        match deserialized.contact {
            Contact::String(email) => assert_eq!(email, "jane@example.com"),
            _ => panic!("Expected string contact"),
        }
    }
}