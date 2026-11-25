# smbCloud for Claude

This plugin connects Claude to your [smbCloud](https://smbcloud.xyz) account. The first part of the account it covers is smbCloud Mail: Claude can read the mail in your inboxes, search it, send messages from an inbox you own, and run a web search, all from the conversation.

## What you can ask

- "What came in overnight on my support inbox?"
- "Find the email from the accountant about the March invoice and summarize it."
- "Reply to the last message in my sales inbox saying we can meet on Thursday."
- "Search the web for the current DMARC recommendations and check them against my domain's setup."

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

The plugin talks only to `https://api.smbcloud.xyz/v1/mcp`, the smbCloud API. Claude sends it the arguments for each tool (an inbox, a search query, the text of a message you ask it to send) and gets back the results (message contents and metadata). The plugin stores nothing on your machine and has no scripts, executables, or hooks.

`web_search` runs on a third-party search provider: smbCloud passes it the search query and the number of results requested, and nothing else. The [privacy policy](https://smbcloud.xyz/privacy-policy) names the provider and says what smbCloud keeps.

`send_message` sends email on your behalf, and the recipient sees it come from your inbox. Check what Claude proposes before you approve it.

## Privacy

smbCloud's privacy policy is at <https://smbcloud.xyz/privacy-policy>.

## Support and license

Issues and questions: <https://github.com/smbcloudXYZ/smbcloud-cli/issues>. Licensed under Apache-2.0.
