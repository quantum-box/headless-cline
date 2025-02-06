use crate::prompts::tools::types::ToolArgs;

#[allow(dead_code)]
pub fn get_access_mcp_resource_description(args: &ToolArgs) -> Option<String> {
    args.mcp_hub?;

    Some(r##"## access_mcp_resource
Description: Request to access a resource provided by a connected MCP server. Resources represent data sources that can be used as context, such as files, API responses, or system information.
Parameters:
- server_name: (required) The name of the MCP server providing the resource
- uri: (required) The URI identifying the specific resource to access
Usage:
<access_mcp_resource>
<server_name>server name here</server_name>
<uri>resource URI here</uri>
</access_mcp_resource>

Example: Requesting to access an MCP resource

<access_mcp_resource>
<server_name>weather-server</server_name>
<uri>weather://san-francisco/current</uri>
</access_mcp_resource>"##.to_string())
}
