use {
    anyhow::Result,
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    solidmcp::{McpServerBuilder, NotificationCtx},
    std::sync::Arc,
};

#[derive(Debug, Clone)]
struct AppContext {
    name: String,
}

#[derive(JsonSchema, Deserialize)]
struct CalculateInput {
    a: f64,
    b: f64,
    operation: String,
}

#[derive(JsonSchema, Serialize)]
struct CalculateOutput {
    result: f64,
    formula: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let context = AppContext {
        name: "Calculator Server".to_string(),
    };

    let mut server = McpServerBuilder::new(context, "calc-server", "1.0.0")
        .with_tool_schemas(
            "calculate",
            "Perform mathematical calculations",
            |input: CalculateInput, _ctx: Arc<AppContext>, notif: NotificationCtx| async move {
                notif.info(&format!("Computing {} {} {}", input.a, input.operation, input.b))?;

                let result = match input.operation.as_str() {
                    "add" => input.a + input.b,
                    "subtract" => input.a - input.b,
                    "multiply" => input.a * input.b,
                    "divide" => {
                        if input.b == 0.0 {
                            return Err(anyhow::anyhow!("Division by zero"));
                        }
                        input.a / input.b
                    }
                    _ => return Err(anyhow::anyhow!("Unknown operation: {}", input.operation)),
                };

                Ok(CalculateOutput {
                    result,
                    formula: format!("{} {} {} = {}", input.a, input.operation, input.b, result),
                })
            },
        )
        .build()
        .await?;

    println!("Starting calculator server with output schema support...");
    server.start(3000).await?;
    Ok(())
}