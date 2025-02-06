use crate::services::diff::DiffStrategy;
use crate::services::mcp::{McpHub, McpServerStatus};
use serde_json::Value;

pub async fn get_mcp_servers_section(
    mcp_hub: Option<&Box<McpHub>>,
    diff_strategy: Option<&Box<dyn DiffStrategy>>,
    enable_mcp_server_creation: Option<bool>,
) -> String {
    if mcp_hub.is_none() {
        return String::new();
    }

    let mcp_hub = mcp_hub.unwrap();
    let connected_servers = if !mcp_hub.get_servers().is_empty() {
        let mut sections = Vec::new();
        for server in mcp_hub.get_servers() {
            if !matches!(server.status, McpServerStatus::Connected) {
                continue;
            }

            let mut server_section = String::new();
            let config: Value = serde_json::from_str(&server.config).unwrap_or_default();
            let command = config["command"].as_str().unwrap_or_default();
            let args = config["args"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();

            server_section.push_str(&format!(
                "## {} (`{}{}`)",
                server.name,
                command,
                if !args.is_empty() {
                    format!(" {}", args)
                } else {
                    String::new()
                }
            ));

            if let Some(tools) = &server.tools {
                let tools_section = tools
                    .iter()
                    .map(|tool| {
                        let schema_str = tool
                            .input_schema
                            .as_ref()
                            .map(|schema| {
                                format!(
                                    "    Input Schema:\n    {}",
                                    serde_json::to_string_pretty(schema)
                                        .unwrap_or_default()
                                        .split('\n')
                                        .collect::<Vec<_>>()
                                        .join("\n    ")
                                )
                            })
                            .unwrap_or_default();

                        format!("- {}: {}\n{}", tool.name, tool.description, schema_str)
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");

                if !tools_section.is_empty() {
                    server_section.push_str("\n\n### Available Tools\n");
                    server_section.push_str(&tools_section);
                }
            }

            if let Some(templates) = &server.resource_templates {
                let templates_section = templates
                    .iter()
                    .map(|template| {
                        format!(
                            "- {} ({}): {}",
                            template.uri_template, template.name, template.description
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                if !templates_section.is_empty() {
                    server_section.push_str("\n\n### Resource Templates\n");
                    server_section.push_str(&templates_section);
                }
            }

            if let Some(resources) = &server.resources {
                let resources_section = resources
                    .iter()
                    .map(|resource| {
                        format!(
                            "- {} ({}): {}",
                            resource.uri, resource.name, resource.description
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                if !resources_section.is_empty() {
                    server_section.push_str("\n\n### Direct Resources\n");
                    server_section.push_str(&resources_section);
                }
            }

            sections.push(server_section);
        }
        sections.join("\n\n")
    } else {
        "(No MCP servers currently connected)".to_string()
    };

    let mut base_section = format!(
        "MCP SERVERS\n\nThe Model Context Protocol (MCP) enables communication between the system and locally running MCP servers that provide additional tools and resources to extend your capabilities.\n\n# Connected MCP Servers\n\nWhen a server is connected, you can use the server's tools via the `use_mcp_tool` tool, and access the server's resources via the `access_mcp_resource` tool.\n\n{}",
        connected_servers
    );

    if !enable_mcp_server_creation.unwrap_or(false) {
        return base_section;
    }

    let servers_path = mcp_hub.get_mcp_servers_path().await;
    let settings_path = mcp_hub.get_mcp_settings_file_path().await;

    base_section.push_str(&format!(
        "\n\n## Creating an MCP Server\n\nThe user may ask you something along the lines of \"add a tool\" that does some function, in other words to create an MCP server that provides tools and resources that may connect to external APIs for example. You have the ability to create an MCP server and add it to a configuration file that will then expose the tools and resources for you to use with `use_mcp_tool` and `access_mcp_resource`.\n\nWhen creating MCP servers, it's important to understand that they operate in a non-interactive environment. The server cannot initiate OAuth flows, open browser windows, or prompt for user input during runtime. All credentials and authentication tokens must be provided upfront through environment variables in the MCP settings configuration. For example, Spotify's API uses OAuth to get a refresh token for the user, but the MCP server cannot initiate this flow. While you can walk the user through obtaining an application client ID and secret, you may have to create a separate one-time setup script (like get-refresh-token.js) that captures and logs the final piece of the puzzle: the user's refresh token (i.e. you might run the script using execute_command which would open a browser for authentication, and then log the refresh token so that you can see it in the command output for you to use in the MCP settings configuration).\n\nUnless the user specifies otherwise, new MCP servers should be created in: {}\n\n### Example MCP Server\n\nFor example, if the user wanted to give you the ability to retrieve weather information, you could create an MCP server that uses the OpenWeather API to get weather information, add it to the MCP settings configuration file, and then notice that you now have access to new tools and resources in the system prompt that you might use to show the user your new capabilities.\n\nThe following example demonstrates how to build an MCP server that provides weather data functionality. While this example shows how to implement resources, resource templates, and tools, in practice you should prefer using tools since they are more flexible and can handle dynamic parameters. The resource and resource template implementations are included here mainly for demonstration purposes of the different MCP capabilities, but a real weather server would likely just expose tools for fetching weather data. (The following steps are for macOS)\n\n1. Use the `create-typescript-server` tool to bootstrap a new project in the default MCP servers directory:\n\n```bash\ncd {}\nnpx @modelcontextprotocol/create-server weather-server\ncd weather-server\n# Install dependencies\nnpm install axios\n```\n\nThis will create a new project with the following structure:\n\n```\nweather-server/\n  ├── package.json\n      {{\n        ...\n        \"type\": \"module\", // added by default, uses ES module syntax (import/export) rather than CommonJS (require/module.exports) (Important to know if you create additional scripts in this server repository like a get-refresh-token.js script)\n        \"scripts\": {{\n          \"build\": \"tsc && node -e \\\"require('fs').chmodSync('build/index.js', '755')\\\"\",\n          ...\n        }}\n        ...\n      }}\n  ├── tsconfig.json\n  └── src/\n      └── weather-server/\n          └── index.ts      # Main server implementation\n```\n\n2. Replace `src/index.ts` with the following:\n\n```typescript\n#!/usr/bin/env node\nimport {{ Server }} from '@modelcontextprotocol/sdk/server/index.js';\nimport {{ StdioServerTransport }} from '@modelcontextprotocol/sdk/server/stdio.js';\nimport {{\n  CallToolRequestSchema,\n  ErrorCode,\n  ListResourcesRequestSchema,\n  ListResourceTemplatesRequestSchema,\n  ListToolsRequestSchema,\n  McpError,\n  ReadResourceRequestSchema,\n}} from '@modelcontextprotocol/sdk/types.js';\nimport axios from 'axios';\n\nconst API_KEY = process.env.OPENWEATHER_API_KEY; // provided by MCP config\nif (!API_KEY) {{\n  throw new Error('OPENWEATHER_API_KEY environment variable is required');\n}}\n\ninterface OpenWeatherResponse {{\n  main: {{\n    temp: number;\n    humidity: number;\n  }};\n  weather: [{{ description: string }}];\n  wind: {{ speed: number }};\n  dt_txt?: string;\n}}\n\nconst isValidForecastArgs = (\n  args: any\n): args is {{ city: string; days?: number }} =>\n  typeof args === 'object' &&\n  args !== null &&\n  typeof args.city === 'string' &&\n  (args.days === undefined || typeof args.days === 'number');\n\nclass WeatherServer {{\n  private server: Server;\n  private axiosInstance;\n\n  constructor() {{\n    this.server = new Server(\n      {{\n        name: 'example-weather-server',\n        version: '0.1.0',\n      }},\n      {{\n        capabilities: {{\n          resources: {{}},\n          tools: {{}},\n        }},\n      }}\n    );\n\n    this.axiosInstance = axios.create({{\n      baseURL: 'http://api.openweathermap.org/data/2.5',\n      params: {{\n        appid: API_KEY,\n        units: 'metric',\n      }},\n    }});\n\n    this.setupResourceHandlers();\n    this.setupToolHandlers();\n    \n    // Error handling\n    this.server.onerror = (error) => console.error('[MCP Error]', error);\n    process.on('SIGINT', async () => {{\n      await this.server.close();\n      process.exit(0);\n    }});\n  }}\n\n  // MCP Resources represent any kind of UTF-8 encoded data that an MCP server wants to make available to clients, such as database records, API responses, log files, and more. Servers define direct resources with a static URI or dynamic resources with a URI template that follows the format `[protocol]://[host]/[path]`.\n  private setupResourceHandlers() {{\n    // For static resources, servers can expose a list of resources:\n    this.server.setRequestHandler(ListResourcesRequestSchema, async () => ({{\n      resources: [\n        // This is a poor example since you could use the resource template to get the same information but this demonstrates how to define a static resource\n        {{\n          uri: `weather://San Francisco/current`, // Unique identifier for San Francisco weather resource\n          name: `Current weather in San Francisco`, // Human-readable name\n          mimeType: 'application/json', // Optional MIME type\n          // Optional description\n          description:\n            'Real-time weather data for San Francisco including temperature, conditions, humidity, and wind speed',\n        }},\n      ],\n    }}));\n\n    // For dynamic resources, servers can expose resource templates:\n    this.server.setRequestHandler(\n      ListResourceTemplatesRequestSchema,\n      async () => ({{\n        resourceTemplates: [\n          {{\n            uriTemplate: 'weather://{{city}}/current', // URI template (RFC 6570)\n            name: 'Current weather for a given city', // Human-readable name\n            mimeType: 'application/json', // Optional MIME type\n            description: 'Real-time weather data for a specified city', // Optional description\n          }},\n        ],\n      }}\n    );\n\n    // ReadResourceRequestSchema is used for both static resources and dynamic resource templates\n    this.server.setRequestHandler(\n      ReadResourceRequestSchema,\n      async (request) => {{\n        const match = request.params.uri.match(\n          /^weather:\\/\\/([^/]+)\\/current$/\n        );\n        if (!match) {{\n          throw new McpError(\n            ErrorCode.InvalidRequest,\n            `Invalid URI format: ${{request.params.uri}}`\n          );\n        }}\n        const city = decodeURIComponent(match[1]);\n\n        try {{\n          const response = await this.axiosInstance.get(\n            'weather', // current weather\n            {{\n              params: {{ q: city }},\n            }}\n          );\n\n          return {{\n            contents: [\n              {{\n                uri: request.params.uri,\n                mimeType: 'application/json',\n                text: JSON.stringify(\n                  {{\n                    temperature: response.data.main.temp,\n                    conditions: response.data.weather[0].description,\n                    humidity: response.data.main.humidity,\n                    wind_speed: response.data.wind.speed,\n                    timestamp: new Date().toISOString(),\n                  }},\n                  null,\n                  2\n                ),\n              }},\n            ],\n          }};\n        }} catch (error) {{\n          if (axios.isAxiosError(error)) {{\n            throw new McpError(\n              ErrorCode.InternalError,\n              `Weather API error: ${{error.response?.data.message ?? error.message}}`\n            );\n          }}\n          throw error;\n        }}\n      }}\n    );\n  }}\n\n  /* MCP Tools enable servers to expose executable functionality to the system. Through these tools, you can interact with external systems, perform computations, and take actions in the real world.\n   * - Like resources, tools are identified by unique names and can include descriptions to guide their usage. However, unlike resources, tools represent dynamic operations that can modify state or interact with external systems.\n   * - While resources and tools are similar, you should prefer to create tools over resources when possible as they provide more flexibility.\n   */\n  private setupToolHandlers() {{\n    this.server.setRequestHandler(ListToolsRequestSchema, async () => ({{\n      tools: [\n        {{\n          name: 'get_forecast', // Unique identifier\n          description: 'Get weather forecast for a city', // Human-readable description\n          inputSchema: {{\n            // JSON Schema for parameters\n            type: 'object',\n            properties: {{\n              city: {{\n                type: 'string',\n                description: 'City name',\n              }},\n              days: {{\n                type: 'number',\n                description: 'Number of days (1-5)',\n                minimum: 1,\n                maximum: 5,\n              }},\n            }},\n            required: ['city'], // Array of required property names\n          }},\n        }},\n      ],\n    }}));\n\n    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {{\n      if (request.params.name !== 'get_forecast') {{\n        throw new McpError(\n          ErrorCode.MethodNotFound,\n          `Unknown tool: ${{request.params.name}}`\n        );\n      }}\n\n      if (!isValidForecastArgs(request.params.arguments)) {{\n        throw new McpError(\n          ErrorCode.InvalidParams,\n          'Invalid forecast arguments'\n        );\n      }}\n\n      const city = request.params.arguments.city;\n      const days = Math.min(request.params.arguments.days || 3, 5);\n\n      try {{\n        const response = await this.axiosInstance.get<{{\n          list: OpenWeatherResponse[];\n        }}>('forecast', {{\n          params: {{\n            q: city,\n            cnt: days * 8,\n          }},\n        }});\n\n        return {{\n          content: [\n            {{\n              type: 'text',\n              text: JSON.stringify(response.data.list, null, 2),\n            }},\n          ],\n        }};\n      }} catch (error) {{\n        if (axios.isAxiosError(error)) {{\n          return {{\n            content: [\n              {{\n                type: 'text',\n                text: `Weather API error: ${{error.response?.data.message ?? error.message}}`,\n              }},\n            ],\n            isError: true,\n          }};\n        }}\n        throw error;\n      }}\n    }});\n  }}\n\n  async run() {{\n    const transport = new StdioServerTransport();\n    await this.server.connect(transport);\n    console.error('Weather MCP server running on stdio');\n  }}\n}}\n\nconst server = new WeatherServer();\nserver.run().catch(console.error);\n```\n\n(Remember: This is just an example–you may use different dependencies, break the implementation up into multiple files, etc.)\n\n3. Build and compile the executable JavaScript file\n\n```bash\nnpm run build\n```\n\n4. Whenever you need an environment variable such as an API key to configure the MCP server, walk the user through the process of getting the key. For example, they may need to create an account and go to a developer dashboard to generate the key. Provide step-by-step instructions and URLs to make it easy for the user to retrieve the necessary information. Then use the ask_followup_question tool to ask the user for the key, in this case the OpenWeather API key.\n\n5. Install the MCP Server by adding the MCP server configuration to the settings file located at '{}'. The settings file may have other MCP servers already configured, so you would read it first and then add your new server to the existing `mcpServers` object.\n\nIMPORTANT: Regardless of what else you see in the MCP settings file, you must default any new MCP servers you create to disabled=false and alwaysAllow=[].\n\n```json\n{{\n  \"mcpServers\": {{\n    ...,\n    \"weather\": {{\n      \"command\": \"node\",\n      \"args\": [\"/path/to/weather-server/build/index.js\"],\n      \"env\": {{\n        \"OPENWEATHER_API_KEY\": \"user-provided-api-key\"\n      }}\n    }},\n  }}\n}}\n```\n\n(Note: the user may also ask you to install the MCP server to the Claude desktop app, in which case you would read then modify `~/Library/Application\\ Support/Claude/claude_desktop_config.json` on macOS for example. It follows the same format of a top level `mcpServers` object.)\n\n6. After you have edited the MCP settings configuration file, the system will automatically run all the servers and expose the available tools and resources in the 'Connected MCP Servers' section.\n\n7. Now that you have access to these new tools and resources, you may suggest ways the user can command you to invoke them - for example, with this new weather tool now available, you can invite the user to ask \"what's the weather in San Francisco?\"\n\n## Editing MCP Servers\n\nThe user may ask to add tools or resources that may make sense to add to an existing MCP server (listed under 'Connected MCP Servers' above: {}, e.g. if it would use the same API. This would be possible if you can locate the MCP server repository on the user's system by looking at the server arguments for a filepath. You might then use list_files and read_file to explore the files in the repository, and use write_to_file{} to make changes to the files.\n\nHowever some MCP servers may be running from installed packages rather than a local repository, in which case it may make more sense to create a new MCP server.\n\n# MCP Servers Are Not Always Necessary\n\nThe user may not always request the use or creation of MCP servers. Instead, they might provide tasks that can be completed with existing tools. While using the MCP SDK to extend your capabilities can be useful, it's important to understand that this is just one specialized type of task you can accomplish. You should only implement MCP servers when the user explicitly requests it (e.g., \"add a tool that...\").\n\nRemember: The MCP documentation and example provided above are to help you understand and work with existing MCP servers or create new ones when requested by the user. You already have access to tools and capabilities that can be used to accomplish a wide range of tasks.",
        servers_path,
        servers_path,
        settings_path,
        mcp_hub
            .get_servers()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        if diff_strategy.is_some() {
            " or apply_diff"
        } else {
            ""
        }
    ));

    base_section
}
