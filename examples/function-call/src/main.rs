use async_openai::{
    types::{
        ChatCompletionRequestFunctionMessageArgs, ChatCompletionRequestUserMessageArgs,
        ChatCompletionToolArgs, CreateChatCompletionRequestArgs, FunctionObjectArgs,
    },
    Client,
};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // This should come from env var outside the program
    std::env::set_var("RUST_LOG", "warn");

    // Setup tracing subscriber so that library can log the rate limited message
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let client = Client::new();

    let model = "gpt-4o-mini";

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u32)
        .model(model)
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content("What's the weather like in Boston?")
            .build()?
            .into()])
        .tools([ChatCompletionToolArgs::default()
            .function(FunctionObjectArgs::default()
                .name("get_current_weather")
                .description("Get the current weather in a given location")
                .parameters(json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city and state, e.g. San Francisco, CA",
                        },
                        "unit": { "type": "string", "enum": ["celsius", "fahrenheit"] },
                    },
                    "required": ["location"],
                }))
                .build()?)
            .build()?])
        .tool_choice("auto")
        .build()?;

    let response_message = client
        .chat()
        .create(request)
        .await?
        .choices
        .first()
        .unwrap()
        .message
        .clone();

    if let Some(tool_calls) = response_message.tool_calls {
        if let Some(tool_call) = tool_calls.first() {
            let mut available_functions: HashMap<&str, fn(&str, &str) -> serde_json::Value> =
                HashMap::new();
            available_functions.insert("get_current_weather", get_current_weather);
            let function_name = &tool_call.function.name;
            let function_args: serde_json::Value = tool_call.function.arguments.parse().unwrap();

            let location = function_args["location"].as_str().unwrap();
            let unit = "fahrenheit";
            let function = available_functions.get(function_name.as_str()).unwrap();
            let function_response = function(location, unit);

            let message = vec![
                ChatCompletionRequestUserMessageArgs::default()
                    .content("What's the weather like in Boston?")
                    .build()?
                    .into(),
                ChatCompletionRequestFunctionMessageArgs::default()
                    .content(function_response.to_string())
                    .name(function_name.clone())
                    .build()?
                    .into(),
            ];

            println!("{}", serde_json::to_string(&message).unwrap());

            let request = CreateChatCompletionRequestArgs::default()
                .max_tokens(512u32)
                .model(model)
                .messages(message)
                .build()?;

            let response = client.chat().create(request).await?;

            println!("\nResponse:\n");
            for choice in response.choices {
                println!(
                    "{}: Role: {}  Content: {:?}",
                    choice.index, choice.message.role, choice.message.content
                );
            }
        }
    }

    Ok(())
}

fn get_current_weather(location: &str, unit: &str) -> serde_json::Value {
    let weather_info = json!({
        "location": location,
        "temperature": "72",
        "unit": unit,
        "forecast": ["sunny", "windy"]
    });

    weather_info
}
