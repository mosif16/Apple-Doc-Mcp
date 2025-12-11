$lines = @(
    '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}',
    '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}',
    '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"React useState hook"}},"id":3}'
)
$lines | & "C:\Users\moham\Desktop\Doc-Mcp\target\release\docs-mcp-cli.exe"
