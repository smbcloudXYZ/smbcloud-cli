# smbCloud for Claude

This plugin connects Claude to your [smbCloud](https://smbcloud.xyz) account. It lets Claude read the mail in your smbCloud Mail inboxes, search it, send messages from an inbox you own, and run a web search, all from the conversation.

## What you can ask

- "What came in overnight on my support inbox?"
- "Find the email from the accountant about the March invoice and summarize it."
- "Reply to the last message in my sales inbox saying we can meet on Thursday."
- "Search the web for the SES sending limits and tell me what applies to a new account."

## Tools

The plugin adds one MCP server, `smbcloud`, with these tools:

| Tool | What it does |
| --- | --- |
| `whoami` | Shows the account you are signed in as. |
| `list_mailboxes` | Lists your inboxes. |
| `list_messages` | Lists received messages in an inbox. |
| `get_message` | Reads one message. |
| `search_messages` | Searches the messages in an inbox. |
| `list_sent_messages` | Lists messages sent from an inbox. |
| `send_message` | Sends an email from one of your inboxes. This sends real mail. |
| `web_search` | Searches the web. |

Claude can only reach inboxes that belong to the account you sign in with.

## Signing in

The first time you use the plugin, Claude opens a smbCloud page in your browser. Sign in, check the app and permissions it shows, and choose Allow. Claude then holds an access token for the `mcp` scope. You can sign out of the connection from Claude at any time.

## What data is sent where

Everything goes to `https://api.smbcloud.xyz/v1/mcp`, the smbCloud API. That includes the arguments Claude passes to a tool (an inbox, a search query, the text of a message you ask it to send) and the results it gets back (message contents and metadata). The plugin sends nothing to any other service and stores nothing on your machine. It has no scripts, executables, or hooks.

`send_message` sends email on your behalf, and the recipient sees it come from your inbox. Check what Claude proposes before you approve it.

## More tools for Claude Code

The hosted server covers mail and web search. If you use Claude Code and want projects, deployments, tenants, and Auth apps as well, install the smbCloud CLI and run it as a local MCP server instead:

```sh
claude mcp add smbcloud-cli -- smb --mcp
```

See the [MCP server guide](https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/docs/mcp.md) for installation and the full tool list.

## Privacy

smbCloud's privacy policy is at <https://smbcloud.xyz/privacy-policy>.

## Support and license

Issues and questions: <https://github.com/smbcloudXYZ/smbcloud-cli/issues>. Licensed under Apache-2.0.
