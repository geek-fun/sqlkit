# AI permissions

SQL database and document database connections share one AI permission chain across built-in Data Studio and external MCP clients. This policy does not grant database privileges or change manual database access.

For each action (read, create data, update data, delete data, manage structure/administration), resolve an explicit connection allow/deny first, otherwise use the system default, otherwise deny. Omitted connection decisions inherit; explicit false is a denial. Presets are shortcuts for these individual decisions.

Data Studio only operates on attached connections. Inherited attachment permissions use the live connection policy, rather than a permission snapshot. Custom conversation restrictions can narrow access, but cannot grant an operation denied by the connection chain. Ask/Auto controls confirmation and never grants CRUD permissions. Backend checks run again before execution, including retries.

An empty set of allowed actions denies access. There is no separate MCP exposure setting or connection allowlist. Connection metadata returned to AI clients excludes inaccessible connections; Data Studio discovery is additionally limited to its attached connections. The MCP server's availability and auto-start preference remain service settings.

Read & modify data includes data deletion but excludes structure and administration. Mixed operations require every affected permission: SQL writes inspect statement tokens, bulk requests inspect every action, and aggregation pipelines that write require write and management access. Risk controls confirmation independently from action permissions.

Legacy allowlists migrate to explicit connection decisions preserving exclusions. When an allowlist existed, new connections default to no access. Existing per-connection restrictions are preserved. Corrupt or unreadable policy files deny execution rather than falling back to permissive settings. Policy writes are atomic and take effect for subsequent calls without restarting the bridge.

Both applications pin the shared agent runtime to an exact Git revision in their Cargo manifests and lockfiles. Each application owns its permission policy and database-operation classification.

The application-owned `ai_permissions` module resolves policy, migrates legacy settings, classifies database payloads, and gates both MCP and built-in agent execution. The shared agent routes requests through an application preparation hook and carries its confirmation risk and context; it does not resolve this permission chain. Execution rechecks the application policy on every attempt.

AI schema reads use the same application gate as tool calls. Cached schema and editor context are included only when the associated connections permit reading. Manual database UI commands retain their existing access behavior. SQLKit does not expose a generic capability invocation command to the WebView.

Agent executors are bound to a conversation and its connection/source identities. Before dispatch they reload connection policy, current attachment restrictions, and confirmation rules. Detaching or deleting a source prevents subsequent dispatches. The sidebar uses its bound active connection without requiring a Data Studio attachment.

Preparation rejects denied calls before displaying confirmation. A queued call whose confirmation requirements become stricter returns to the existing approval flow. Permission denials are terminal and do not trigger transport retries; approval never bypasses a later denial. Rechecks occur at dispatch and do not cancel operations already dispatched or erase context already sent to a model.

The host-hook API is introduced by [data-studio-agent #35](https://github.com/geek-fun/data-studio-agent/pull/35). Merge that runtime implementation before merging its consumers. Keep the exact compatible revision until a release containing these hooks exists; after the runtime is merged and released, update both the manifest and lockfile to that release. Older release tags do not provide this API.

## Permission controls

Settings separate MCP service controls from the AI defaults shared by Data Studio and external MCP clients. Access levels use the existing settings radio pattern; a short description below the option group updates with the selected access level. Custom is an access choice that reveals individual actions; it edits the same values as the presets. Choosing Custom alone does not change permissions.

Connection dialogs group the agent prompt with the AI permissions section, separate from the SSL and SSH tunnel blocks. They default to system inheritance. Custom actions use switches: inherited actions show Use default (with the current decision), explicit actions show their decision with the connection source, and toggling sets an explicit decision. Within the connection section, saved custom combinations show their action controls automatically. Choosing a preset sets explicit decisions for every action; choosing Use system default clears them.

Editing AI permission defaults and per-connection decisions requires the existing local Ultimate entitlement, including eligible version-locked releases. Community users see the existing upgrade prompt in connection dialogs. Ordinary connection edits preserve saved decisions. Permission cleanup after deletion remains available only when the connection no longer exists; it cannot reset the permissions of a saved connection.

## Blocked actions and conversation history

Ask and Auto both run permitted reads automatically. Ask requests approval for ordinary data writes; Auto runs them automatically. Both modes request approval for permitted deletes and structure/administration changes unless a saved confirmation rule applies. The application displays the approval card when the agent submits a tool call; model instructions do not replace that gate. Approval does not grant an action denied by the connection chain or conversation restrictions.

A policy denial records the tool result and pauses the agent through the existing runtime cancellation boundary. The host persists the conversation as blocked and reports that state after the run has settled. Already dispatched work is not undone. Other unresolved calls show an unavailable outcome instead of success. Explicit user rejection remains distinct from a policy block.

Each denied call retains a visible notice with its connection, required action, and decision source when recorded. For connection access denials, Review permissions always edits the blocked connection, including when its denied decision is inherited from system defaults. It never changes global defaults from chat. Conversation restrictions and saved confirmation rules retain their own recovery controls. Save and Back to chat share one dialog footer row. Saving never executes the blocked action. Retry request starts a new agent turn with the original tool arguments, fresh permissions, and normal confirmation; it does not replay earlier successful work automatically or bypass the application gate.

History loads persisted tool-call statuses and correlated results rather than assuming every call succeeded. Denials and failures remain visible after reopening. Interrupted or missing outcomes are shown as unavailable; legacy records whose permission source was never recorded retain a generic permission notice.
