# Tool Registration and Execution

## Tool Registration Patterns

### Basic Tool Registration
```typescript
interface Tool {
  name: string;
  description: string;
  inputSchema: {
    type: "object";
    properties: Record<string, JSONSchemaProperty>;
    required?: string[];
  };
}

// Example: Simple calculation tool
const calculatorTool: Tool = {
  name: "calculate",
  description: "Perform arithmetic calculations",
  inputSchema: {
    type: "object",
    properties: {
      expression: {
        type: "string",
        description: "Mathematical expression to evaluate",
        pattern: "^[0-9+\\-*/().\\s]+$"
      }
    },
    required: ["expression"]
  }
};
```

### Tool with Complex Input Schema
```typescript
const databaseQueryTool: Tool = {
  name: "query_database",
  title: "Database Query Executor",
  description: "Execute SQL queries against the database",
  inputSchema: {
    type: "object",
    properties: {
      query: {
        type: "string",
        description: "SQL query to execute",
        maxLength: 1000
      },
      database: {
        type: "string", 
        enum: ["users", "products", "orders"],
        description: "Target database"
      },
      limit: {
        type: "integer",
        minimum: 1,
        maximum: 100,
        default: 10,
        description: "Maximum rows to return"
      },
      parameters: {
        type: "array",
        items: {
          type: "string"
        },
        description: "Query parameters for prepared statements"
      }
    },
    required: ["query", "database"]
  },
  annotations: {
    title: "SQL Query Tool",
    readOnlyHint: true,
    openWorldHint: false
  }
};
```

## Tool Execution Patterns

### Synchronous Execution
```typescript
async function executeCalculatorTool(args: any): Promise<ToolResult> {
  try {
    // Input validation (already done by framework)
    const { expression } = args;
    
    // Execute the calculation
    const result = evaluateExpression(expression);
    
    return {
      content: [
        {
          type: "text",
          text: `${expression} = ${result}`
        }
      ],
      isError: false
    };
  } catch (error) {
    return {
      content: [
        {
          type: "text", 
          text: `Calculation error: ${error.message}`
        }
      ],
      isError: true
    };
  }
}
```

### Asynchronous Execution with Progress
```typescript
async function executeDataProcessingTool(
  args: any, 
  progressCallback?: (progress: number, message?: string) => void
): Promise<ToolResult> {
  
  const { filePath, operation } = args;
  
  try {
    progressCallback?.(0, "Starting data processing...");
    
    // Step 1: Load data
    const data = await loadDataFile(filePath);
    progressCallback?.(25, "Data loaded successfully");
    
    // Step 2: Validate data
    const validData = await validateData(data);
    progressCallback?.(50, "Data validation complete");
    
    // Step 3: Process data
    const results = await processData(validData, operation);
    progressCallback?.(75, "Data processing complete");
    
    // Step 4: Generate report
    const report = await generateReport(results);
    progressCallback?.(100, "Report generated");
    
    return {
      content: [
        {
          type: "text",
          text: `Processed ${data.length} records successfully`,
          annotations: {
            audience: ["user"],
            priority: 0.9
          }
        },
        {
          type: "resource_link",
          uri: `file://${report.path}`,
          name: "processing_report.json",
          description: "Detailed processing report",
          mimeType: "application/json"
        }
      ],
      isError: false
    };
    
  } catch (error) {
    return {
      content: [
        {
          type: "text",
          text: `Processing failed: ${error.message}`
        }
      ],
      isError: true
    };
  }
}
```

### Tool with Multiple Output Types
```typescript
async function executeImageAnalysisTool(args: any): Promise<ToolResult> {
  const { imageUrl, analysisType } = args;
  
  try {
    const analysis = await analyzeImage(imageUrl, analysisType);
    
    const content: ContentBlock[] = [
      // Text summary
      {
        type: "text",
        text: `Image analysis complete. Found ${analysis.objects.length} objects.`,
        annotations: {
          audience: ["user"],
          priority: 0.8
        }
      }
    ];
    
    // Add annotated image if available
    if (analysis.annotatedImage) {
      content.push({
        type: "image",
        data: analysis.annotatedImage.base64,
        mimeType: "image/png",
        annotations: {
          audience: ["user", "assistant"],
          priority: 0.9
        }
      });
    }
    
    // Add detailed results as resource
    content.push({
      type: "resource",
      resource: {
        uri: `analysis://${analysis.id}`,
        title: "Detailed Analysis Results",
        mimeType: "application/json",
        text: JSON.stringify(analysis.details, null, 2)
      }
    });
    
    return { content, isError: false };
    
  } catch (error) {
    return {
      content: [
        {
          type: "text",
          text: `Image analysis failed: ${error.message}`
        }
      ],
      isError: true
    };
  }
}
```

## Input Validation Patterns

### Schema-Based Validation
```typescript
function validateToolInput(input: any, schema: JSONSchema): ValidationResult {
  const ajv = new Ajv({ allErrors: true });
  const validate = ajv.compile(schema);
  
  if (!validate(input)) {
    return {
      valid: false,
      errors: validate.errors?.map(err => ({
        field: err.instancePath,
        message: err.message,
        value: err.data
      })) || []
    };
  }
  
  return { valid: true };
}

// Usage in tool execution
async function executeValidatedTool(args: any, schema: JSONSchema): Promise<ToolResult> {
  const validation = validateToolInput(args, schema);
  
  if (!validation.valid) {
    return {
      content: [
        {
          type: "text",
          text: `Invalid arguments: ${validation.errors.map(e => e.message).join(', ')}`
        }
      ],
      isError: true
    };
  }
  
  // Proceed with execution...
}
```

### Custom Validation Logic
```typescript
async function executeFileOperationTool(args: any): Promise<ToolResult> {
  const { operation, filePath, content } = args;
  
  // Custom validation beyond schema
  if (operation === 'write' && !content) {
    return {
      content: [
        {
          type: "text",
          text: "Content is required for write operations"
        }
      ],
      isError: true
    };
  }
  
  // Check file permissions
  if (!await hasFilePermission(filePath, operation)) {
    return {
      content: [
        {
          type: "text",
          text: `Permission denied for ${operation} on ${filePath}`
        }
      ],
      isError: true
    };
  }
  
  // Execute operation...
}
```

## Error Handling Best Practices

### Structured Error Response
```typescript
interface ToolError {
  type: 'validation' | 'permission' | 'network' | 'internal';
  message: string;
  details?: any;
}

function createErrorResult(error: ToolError): ToolResult {
  return {
    content: [
      {
        type: "text",
        text: error.message,
        annotations: {
          audience: ["user"],
          priority: 1.0
        }
      }
    ],
    isError: true,
    // Include error details in structured content for debugging
    structuredContent: {
      error: {
        type: error.type,
        message: error.message,
        details: error.details,
        timestamp: new Date().toISOString()
      }
    }
  };
}
```

### Graceful Degradation
```typescript
async function executeWebScrapeTool(args: any): Promise<ToolResult> {
  const { url, selector } = args;
  
  try {
    const content = await scrapeWebsite(url, selector);
    
    return {
      content: [
        {
          type: "text",
          text: content,
          annotations: {
            audience: ["assistant"],
            priority: 0.8
          }
        }
      ],
      isError: false
    };
    
  } catch (error) {
    // Attempt fallback methods
    if (error.code === 'BLOCKED') {
      try {
        const fallbackContent = await scrapeWithFallback(url);
        return {
          content: [
            {
              type: "text",
              text: `${fallbackContent}\n\nNote: Used fallback method due to access restrictions.`
            }
          ],
          isError: false
        };
      } catch (fallbackError) {
        // Return original error if fallback also fails
      }
    }
    
    return createErrorResult({
      type: 'network',
      message: `Failed to scrape ${url}: ${error.message}`,
      details: { url, selector, errorCode: error.code }
    });
  }
}
```

## Performance Considerations

### Timeout Handling
```typescript
async function executeWithTimeout<T>(
  operation: () => Promise<T>,
  timeoutMs: number = 30000
): Promise<T> {
  
  return Promise.race([
    operation(),
    new Promise<never>((_, reject) => 
      setTimeout(() => reject(new Error('Operation timed out')), timeoutMs)
    )
  ]);
}

async function executeLongRunningTool(args: any): Promise<ToolResult> {
  try {
    const result = await executeWithTimeout(
      () => performComplexOperation(args),
      60000 // 60 second timeout
    );
    
    return {
      content: [{ type: "text", text: result }],
      isError: false
    };
    
  } catch (error) {
    if (error.message === 'Operation timed out') {
      return {
        content: [
          {
            type: "text",
            text: "Operation timed out. Please try with a smaller dataset or contact support."
          }
        ],
        isError: true
      };
    }
    throw error;
  }
}
```

### Resource Cleanup
```typescript
class ToolExecutionContext {
  private resources: Array<() => Promise<void>> = [];
  
  addCleanup(cleanup: () => Promise<void>) {
    this.resources.push(cleanup);
  }
  
  async cleanup() {
    await Promise.all(this.resources.map(cleanup => cleanup()));
  }
}

async function executeDatabaseTool(args: any): Promise<ToolResult> {
  const context = new ToolExecutionContext();
  
  try {
    const connection = await createDatabaseConnection();
    context.addCleanup(() => connection.close());
    
    const result = await connection.query(args.query);
    
    return {
      content: [
        {
          type: "text",
          text: `Query returned ${result.length} rows`
        }
      ],
      isError: false
    };
    
  } finally {
    await context.cleanup();
  }
}
```