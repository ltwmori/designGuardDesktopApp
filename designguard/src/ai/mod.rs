pub mod claude;
pub mod prompts;
pub mod provider;
pub mod ollama;
pub mod router;
pub mod classifier;

// Re-export for convenience
pub use claude::*;
pub use prompts::*;
pub use provider::{AIProvider, SchematicContext, ModelInfo, ProviderStatus, ComponentDetail};
pub use ollama::OllamaClient;
pub use router::AIRouter;
pub use classifier::{
    ComponentRoleClassifier, 
    ComponentRole, 
    ComponentInput, 
    ClassificationResult
};
