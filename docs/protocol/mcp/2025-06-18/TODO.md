# MCP 2025-06-18 Protocol Extraction TODO

## Objective
Extract and reformat the entire MCP 2025-06-18 specification into an information-dense format optimized for LLM consumption and human quick reference. The goal is to create a comprehensive implementation guide that captures all protocol nuances, edge cases, and implementation details while being token-efficient.

## Target Format Structure

### Proposed Directory Structure
```
docs/protocol/mcp/2025-06-18/
├── TODO.md (this file)
├── README.md                    # Master index with navigation guide
├── core/                        # Core protocol mechanics
│   ├── README.md               # Index: types, messages, lifecycle, errors
│   ├── types.md                # All TypeScript interfaces and schemas
│   ├── messages.md             # JSON-RPC message formats
│   ├── lifecycle.md            # Connection lifecycle and states
│   └── errors.md               # Error codes and handling
├── transport/                   # Transport layer details
│   ├── README.md               # Index: transport selection, features
│   ├── websocket.md            # WebSocket implementation
│   ├── http.md                 # HTTP implementation
│   └── headers.md              # Protocol headers (MCP-Protocol-Version)
├── server/                      # Server-side features
│   ├── README.md               # Index: tools, resources, prompts overview
│   ├── capabilities.md         # Server capability declaration
│   ├── tools/                  # Tool system
│   │   ├── README.md          # Tool overview and quick reference
│   │   ├── basics.md          # Registration, input validation
│   │   ├── structured-output.md # NEW: Structured output support
│   │   └── resource-links.md   # NEW: Resource links in results
│   ├── resources/              # Resource system
│   │   ├── README.md          # Resource overview
│   │   ├── types.md           # Text vs blob resources
│   │   ├── subscriptions.md  # Update notifications
│   │   └── uri-schemes.md     # URI format and schemes
│   └── prompts/                # Prompt system
│       ├── README.md          # Prompt overview
│       ├── templates.md       # Template format and variables
│       └── arguments.md       # Argument handling
├── client/                      # Client-side features
│   ├── README.md               # Index: sampling, elicitation, roots
│   ├── capabilities.md         # Client capability declaration
│   ├── sampling/               # LLM sampling
│   │   ├── README.md          # Sampling overview
│   │   ├── requests.md        # Request format
│   │   └── security.md        # Security considerations
│   ├── elicitation/            # NEW: User elicitation
│   │   ├── README.md          # Elicitation overview
│   │   ├── types.md           # Request types
│   │   └── ui-requirements.md # UI implementation guide
│   └── roots/                  # Roots discovery
│       ├── README.md          # Roots overview and use cases
│       └── implementation.md  # Request/response format
├── security/                    # Security and authorization
│   ├── README.md               # Security overview and principles
│   ├── oauth.md                # OAuth Resource Server (NEW)
│   ├── authorization.md        # Authorization flows
│   ├── best-practices.md       # Security best practices
│   └── rfc8707.md             # Resource Indicators (NEW)
├── utilities/                   # Supporting utilities
│   ├── README.md               # Utilities overview
│   ├── progress.md             # Progress tracking
│   ├── cancellation.md         # Request cancellation
│   ├── ping.md                 # Keepalive mechanism
│   ├── logging.md              # Logging levels and format
│   ├── completion.md           # Completion support
│   └── pagination.md           # Pagination patterns
├── reference/                   # Quick reference materials
│   ├── README.md               # Reference overview
│   ├── schemas/                # Schema reference
│   │   ├── README.md          # Schema overview
│   │   ├── complete-types.md  # All TypeScript interfaces
│   │   └── validation.md      # Validation rules
│   ├── error-codes.md          # Complete error code table
│   ├── capability-matrix.md    # Feature/capability matrix
│   ├── message-catalog.md      # All message types reference
│   └── diagrams/               # Visual protocol diagrams
│       ├── README.md          # Diagram index
│       ├── lifecycle.mermaid  # Connection lifecycle flow
│       ├── architecture.mermaid # System architecture
│       ├── message-flow.mermaid # Request/response patterns
│       └── state-machines.mermaid # Protocol state diagrams
└── implementation/              # Implementation guidance
    ├── README.md               # Implementation overview
    ├── quick-start.md          # Getting started guide
    ├── examples/               # Code examples
    │   ├── README.md          # Examples overview
    │   ├── minimal-server.md  # Minimal server implementation
    │   ├── minimal-client.md  # Minimal client implementation
    │   └── common-patterns.md # Common implementation patterns
    ├── migration-guide.md      # Migration from 2025-03-26
    ├── changelog.md            # Detailed changelog
    └── edge-cases.md          # Implementation pitfalls
```

### Format Guidelines
- **Dense Code Blocks**: Use TypeScript interfaces and JSON examples liberally
- **Implementation Tables**: Quick reference tables for message types, error codes, etc.
- **Edge Case Callouts**: Clear ⚠️ WARNING and 📝 NOTE sections for gotchas
- **Minimal Prose**: Bullet points over paragraphs
- **Cross-References**: Link between related concepts using anchors

## Task Breakdown

### Phase 1: Setup and Master Index
- [ ] **Task 1.1**: Create directory structure
  - Create all directories as specified above
  - Initialize README.md files with basic structure

- [ ] **Task 1.2**: Create master README.md index
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/index.mdx`
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/architecture/index.mdx`
  - Protocol overview
  - Feature matrix
  - Navigation guide to all sections
  - Quick decision tree for implementers

### Phase 2: Core Protocol Documentation
- [ ] **Task 2.1**: Create core/README.md
  - Overview of core protocol concepts
  - Links to types, messages, lifecycle, errors

- [ ] **Task 2.2**: Extract core/types.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.ts`
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/schema.mdx`
  - All TypeScript interfaces
  - Type hierarchy diagram
  - Required vs optional fields
  - Validation constraints

- [ ] **Task 2.3**: Document core/messages.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/index.mdx`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json`
  - JSON-RPC 2.0 format
  - Request/Response/Notification patterns
  - Message examples for each type
  - _meta field usage

- [ ] **Task 2.4**: Extract core/lifecycle.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/lifecycle.mdx`
  - Initialize sequence
  - Capability negotiation
  - State machine diagram
  - Shutdown and re-initialization
  - **Create Mermaid diagram**: Connection lifecycle state machine

- [ ] **Task 2.5**: Compile core/errors.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json` (error codes)
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/index.mdx`
  - Complete error code table
  - Error response format
  - Recovery strategies

### Phase 3: Transport Layer Documentation
- [ ] **Task 3.1**: Create transport/README.md
  - Transport overview and selection guide
  - Feature comparison table

- [ ] **Task 3.2**: Document transport/websocket.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/transports.mdx`
  - WebSocket connection lifecycle
  - Message framing
  - Connection persistence
  - Error handling
  - **Create Mermaid diagram**: WebSocket message flow

- [ ] **Task 3.3**: Document transport/http.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/transports.mdx`
  - HTTP request/response cycle
  - Session management
  - Chunked encoding rules
  - Size limits
  - **Create Mermaid diagram**: HTTP session flow with cookies

- [ ] **Task 3.4**: Extract transport/headers.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/changelog.mdx`
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/transports.mdx`
  - MCP-Protocol-Version header (NEW)
  - Content-Type requirements
  - Custom headers

### Phase 4: Server Features Documentation
- [ ] **Task 4.1**: Create server/README.md
  - Server feature overview
  - Capability declaration guide
  - Feature selection matrix

- [ ] **Task 4.2**: Document server/capabilities.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/index.mdx`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json` (ServerCapabilities)
  - Capability declaration format
  - Feature flags
  - Version negotiation

- [ ] **Task 4.3**: Create server/tools/README.md
  - Tools system overview
  - Quick implementation checklist

- [ ] **Task 4.4**: Extract server/tools/basics.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/tools.mdx`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json` (Tool, ToolCallRequest)
  - Tool registration
  - Input schema validation
  - Basic tool implementation

- [ ] **Task 4.5**: Document server/tools/structured-output.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/changelog.mdx`
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/tools.mdx`
  - NEW feature in 2025-06-18
  - Structured content types
  - Implementation examples

- [ ] **Task 4.6**: Document server/tools/resource-links.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/changelog.mdx`
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/tools.mdx`
  - NEW feature in 2025-06-18
  - Resource link format
  - Use cases

- [ ] **Task 4.7**: Create server/resources/README.md
  - Resources system overview
  - Quick reference

- [ ] **Task 4.8**: Extract server/resources/types.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/resources.mdx`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json` (Resource types)
  - Text vs blob resources
  - Content encoding
  - Size limits

- [ ] **Task 4.9**: Document server/resources/subscriptions.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/resources.mdx`
  - Subscription mechanism
  - Update notifications
  - Unsubscribe flow

- [ ] **Task 4.10**: Extract server/resources/uri-schemes.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/resources.mdx`
  - URI format specification
  - Supported schemes
  - Custom scheme guidelines

- [ ] **Task 4.11**: Create server/prompts/README.md
  - Prompts system overview

- [ ] **Task 4.12**: Extract server/prompts/templates.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/prompts.mdx`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json` (Prompt types)
  - Template format
  - Variable substitution
  - Message construction

- [ ] **Task 4.13**: Document server/prompts/arguments.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/prompts.mdx`
  - Argument definition
  - Validation
  - Dynamic arguments

### Phase 5: Client Features Documentation
- [ ] **Task 5.1**: Create client/README.md
  - Client feature overview
  - Capability requirements
  - Implementation guide

- [ ] **Task 5.2**: Document client/capabilities.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/client/index.mdx`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json` (ClientCapabilities)
  - Client capability declaration
  - Feature negotiation

- [ ] **Task 5.3**: Create client/sampling/README.md
  - Sampling overview
  - Security model

- [ ] **Task 5.4**: Extract client/sampling/requests.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/client/sampling.mdx`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json` (CreateMessageRequest)
  - Request format
  - Response handling
  - Example flows

- [ ] **Task 5.5**: Document client/sampling/security.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/client/sampling.mdx`
  - User consent requirements
  - Prompt visibility restrictions
  - Security considerations

- [ ] **Task 5.6**: Create client/elicitation/README.md
  - NEW feature overview
  - Use cases

- [ ] **Task 5.7**: Extract client/elicitation/types.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/client/elicitation.mdx`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json` (ElicitationRequest)
  - Request types (text, choice, confirmation)
  - Response formats

- [ ] **Task 5.8**: Document client/elicitation/ui-requirements.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/client/elicitation.mdx`
  - UI implementation guidelines
  - User experience best practices

- [ ] **Task 5.9**: Create client/roots/README.md
  - Roots feature overview

- [ ] **Task 5.10**: Extract client/roots/implementation.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/client/roots.mdx`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json` (RootsListRequest)
  - Request/response format
  - Use cases
  - Implementation notes

### Phase 6: Security Documentation
- [ ] **Task 6.1**: Create security/README.md
  - Security overview
  - Key principles
  - Implementation checklist

- [ ] **Task 6.2**: Extract security/oauth.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/authorization.mdx`
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/changelog.mdx`
  - OAuth Resource Server classification (NEW)
  - Protected resource metadata
  - Token handling

- [ ] **Task 6.3**: Document security/authorization.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/authorization.mdx`
  - Authorization flows
  - Client credentials
  - Scope management

- [ ] **Task 6.4**: Extract security/best-practices.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/security_best_practices.mdx`
  - Security principles
  - Implementation guidelines
  - Common vulnerabilities

- [ ] **Task 6.5**: Document security/rfc8707.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/changelog.mdx`
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/authorization.mdx`
  - Resource Indicators (NEW)
  - Implementation requirements
  - Security benefits

### Phase 7: Utilities Documentation
- [ ] **Task 7.1**: Create utilities/README.md
  - Utilities overview
  - Feature matrix

- [ ] **Task 7.2**: Extract utilities/progress.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/utilities/progress.mdx`
  - Progress token format
  - Update mechanism
  - UI integration

- [ ] **Task 7.3**: Document utilities/cancellation.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/utilities/cancellation.mdx`
  - Cancellation protocol
  - Request IDs
  - Error handling

- [ ] **Task 7.4**: Extract utilities/ping.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/utilities/ping.mdx`
  - Keepalive mechanism
  - Timeout handling
  - Connection health

- [ ] **Task 7.5**: Document utilities/logging.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/utilities/logging.mdx`
  - Log levels
  - Message format
  - Debug information

- [ ] **Task 7.6**: Extract utilities/completion.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/utilities/completion.mdx`
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/changelog.mdx` (context field)
  - Completion request format
  - Context support (NEW)
  - Response handling

- [ ] **Task 7.7**: Document utilities/pagination.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/utilities/pagination.mdx`
  - Pagination patterns
  - Cursor format
  - Best practices

### Phase 8: Reference Documentation
- [ ] **Task 8.1**: Create reference/README.md
  - Reference overview
  - Quick lookup guide

- [ ] **Task 8.2**: Create reference/schemas/README.md
  - Schema overview
  - Type hierarchy

- [ ] **Task 8.3**: Extract reference/schemas/complete-types.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json`
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.ts`
  - All TypeScript interfaces
  - Enums and constants
  - Type relationships

- [ ] **Task 8.4**: Document reference/schemas/validation.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json`
  - Validation rules
  - Required fields
  - Format constraints

- [ ] **Task 8.5**: Compile reference/error-codes.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json`
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/basic/index.mdx`
  - Complete error code table
  - Error categories
  - Recovery strategies

- [ ] **Task 8.6**: Create reference/capability-matrix.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json` (Capabilities types)
  - Server capabilities
  - Client capabilities
  - Feature dependencies

- [ ] **Task 8.7**: Generate reference/message-catalog.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json`
  - All message types
  - Request/response pairs
  - Notification types

### Phase 9: Implementation Guidance
- [ ] **Task 9.1**: Create implementation/README.md
  - Implementation overview
  - Getting started

- [ ] **Task 9.2**: Write implementation/quick-start.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/index.mdx`
  - Minimal setup
  - Hello world example
  - Common patterns

- [ ] **Task 9.3**: Create implementation/examples/README.md
  - Examples overview
  - Language coverage

- [ ] **Task 9.4**: Document implementation/examples/minimal-server.md
  - Minimal server code
  - Basic features
  - Extension points

- [ ] **Task 9.5**: Document implementation/examples/minimal-client.md
  - Minimal client code
  - Connection setup
  - Basic operations

- [ ] **Task 9.6**: Extract implementation/examples/common-patterns.md
  - Error handling patterns
  - Async operations
  - State management

- [ ] **Task 9.7**: Create implementation/migration-guide.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/changelog.mdx`
  - Breaking changes
  - Migration steps
  - Compatibility layer

- [ ] **Task 9.8**: Extract implementation/changelog.md
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/changelog.mdx`
  - Detailed changes
  - New features
  - Removed features

- [ ] **Task 9.9**: Document implementation/edge-cases.md
  - Common pitfalls
  - Performance tips
  - Debugging strategies

### Phase 10: Visual Documentation and Diagrams
- [ ] **Task 10.1**: Create reference/diagrams/README.md
  - Diagram index and guide
  - Mermaid rendering instructions

- [ ] **Task 10.2**: Extract existing diagrams
  - **Source Files**:
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/resource-picker.png`
    - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/server/slash-command.png`
    - `/Users/anon/dev/modelcontextprotocol/docs/images/mcp-simple-diagram.png`
    - `/Users/anon/dev/modelcontextprotocol/docs/images/java/java-mcp-client-architecture.jpg`
    - `/Users/anon/dev/modelcontextprotocol/docs/images/java/java-mcp-server-architecture.jpg`
    - `/Users/anon/dev/modelcontextprotocol/docs/images/java/mcp-stack.svg`
    - `/Users/anon/dev/modelcontextprotocol/docs/images/java/class-diagrams.puml`
  - Convert images to Mermaid diagrams
  - Extract PlantUML content if useful
  - Note UI/UX patterns from screenshots

- [ ] **Task 10.3**: Create reference/diagrams/lifecycle.mermaid
  - Connection initialization flow
  - State transitions
  - Re-initialization handling
  - Error states

- [ ] **Task 10.4**: Create reference/diagrams/architecture.mermaid
  - Host/Client/Server relationships
  - Transport layer abstraction
  - Feature capabilities map

- [ ] **Task 10.5**: Create reference/diagrams/message-flow.mermaid
  - Request/Response patterns
  - Notification flows
  - Progress/Cancellation sequences
  - Error propagation

- [ ] **Task 10.6**: Create reference/diagrams/state-machines.mermaid
  - Protocol state machine
  - Resource subscription states
  - Tool execution states

- [ ] **Task 10.7**: Add inline diagrams to docs
  - Embed diagrams in relevant sections
  - Create sequence diagrams for complex flows
  - Add class diagrams for type relationships

### Phase 11: Final Polish and Validation
- [ ] **Task 11.1**: Cross-reference validation
  - Verify all types documented
  - Check all features covered
  - Validate internal links

- [ ] **Task 11.2**: Token optimization
  - Remove redundancy
  - Consolidate content
  - Prefer tables over prose

- [ ] **Task 11.3**: Create navigation indices
  - Update all README files
  - Add cross-references
  - Create breadcrumbs

## Success Criteria
- Complete coverage of all 2025-06-18 specification features
- All new features clearly marked
- Implementation-ready reference with code examples
- Quick lookup capability for any protocol detail
- Token-efficient format suitable for LLM context
- No critical implementation details omitted

## Additional Source References
- **Examples and Tutorials**:
  - `/Users/anon/dev/modelcontextprotocol/docs/docs/tutorials/` - Implementation examples
  - `/Users/anon/dev/modelcontextprotocol/docs/docs/sdk.mdx` - SDK usage patterns
- **Architecture Details**:
  - `/Users/anon/dev/modelcontextprotocol/docs/specification/2025-06-18/architecture/index.mdx`
- **Schema Definitions**:
  - Primary: `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.ts`
  - JSON: `/Users/anon/dev/modelcontextprotocol/schema/2025-06-18/schema.json`

## Notes
- Priority on implementation details over conceptual explanations
- Include actual JSON/TypeScript examples wherever possible
- Highlight breaking changes and new features prominently
- Focus on "how to implement" rather than "why it works this way"
- Each README.md should serve as a navigation hub for its section
- Use dense formatting with tables and code blocks for LLM efficiency
- Mark all 2025-06-18 new features with "NEW" badges

## Diagram Guidelines
- Use Mermaid format for all diagrams (LLM-readable)
- Include sequence diagrams for all major protocol flows
- Create state diagrams for lifecycle and connection states
- Add architecture diagrams showing component relationships
- Embed diagrams inline in relevant documentation sections
- Keep diagrams focused and avoid over-complexity
- Add labels and annotations for clarity