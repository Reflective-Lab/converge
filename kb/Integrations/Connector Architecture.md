---
tags: [architecture, integrations]
source: mixed
---
# Connector Architecture

How external system connectors plug into Converge and flow up to higher layers.

## Converge's Role

Converge owns **tool discovery and execution infrastructure**. It does not own
domain semantics or business logic around external systems — that belongs to
layers above (Organism, applications).

## ToolSource — The Connector Bridge

```rust
pub enum ToolSource {
    Mcp { server_name: String, server_uri: String },
    OpenApi { spec_path: String, operation_id: String, method: String, path: String },
    GraphQl { endpoint: String, operation_name: String, operation_type: GraphQlOperationType },
    Inline,
}
```

Community-built connectors register through one of these mechanisms. Converge
discovers and exposes them as `ToolDefinition` instances available to any
consumer (including Organism's planning loop).

## Three Integration Paths

### MCP (Model Context Protocol)

Dynamic tool discovery. An MCP server exposes tools via JSON-RPC. Converge
discovers them at runtime.

```rust
let mcp = McpClient::new("salesforce", McpTransport::Http { url });
let tools = mcp.list_tools()?;
```

Best for: tool servers exposing many actions, agent-selected tooling.

### OpenAPI

Static tool generation from REST specs. Each operation becomes a tool.

Best for: well-documented REST APIs with stable contracts.

### GraphQL

Schema introspection generates tools from queries/mutations.

Best for: systems with rich type systems and nested data.

## What Converge Does NOT Do

- Interpret tool results semantically (that's Organism's job)
- Decide which tools to call (that's the planning loop above)
- Own domain-specific port traits (Organism owns those)
- Build connector SDKs for specific vendors

## What Converge DOES Do

- Discover available tools from MCP/OpenAPI/GraphQL sources
- Execute tool calls via `ToolHandler`
- Enforce authority at the commit boundary (tool execution ≠ fact promotion)
- Track tool availability and health

## The Three-Tier Model (Cross-Stack)

| Tier | What | Owner |
|------|------|-------|
| Tool (Tier 1) | Generic connector, ecosystem-built | Converge discovers, executes |
| Port (Tier 2) | Domain-semantic typed trait | Organism defines, implements |
| Provider (Tier 3) | Interchangeable AI/ML backend | Converge defines, implements |

Converge owns Tier 1 infrastructure and Tier 3 entirely. It has no opinion
about Tier 2 — those are contracts defined above.

## Ecosystem Consumption Pattern

```
Community MCP Server (e.g. Salesforce, SAP, Jira)
    ↓ registers
Converge ToolRegistry
    ↓ exposes ToolDefinition
Organism planning loop (or any consumer of converge-kernel)
    ↓ plans tool usage
Converge commit boundary (authority check)
    ↓ executes
ToolHandler dispatches to MCP/REST/GraphQL
    ↓ returns
Observation recorded
```

## Strategic Intent: API-Only Infrastructure

Converge is **API-only infrastructure**. The connector architecture is a direct
expression of this:

- ToolRegistry is an API surface — consumers discover and invoke tools via gRPC/REST
- MCP, OpenAPI, GraphQL are all standard protocol integrations — no proprietary format
- Converge never wraps external systems in product-specific UIs
- The entire value prop is: governed, auditable API infrastructure that others compose

This means adding a legacy system connector is the same as adding any other API
consumer or provider. No special SDK, no vendor lock-in, no monolithic product.

## Adding a New Connector

1. Find or build an MCP server / OpenAPI spec for the target system
2. Register it in the ToolRegistry configuration
3. Tools become available to any consumer of the kernel
4. No code changes in Converge or Organism required

See also: [[Integrations/MCP Tools]], [[Architecture/Ports]], [[Architecture/Hexagonal Architecture]]
