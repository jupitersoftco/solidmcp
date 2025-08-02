# NEW: Structured Tool Output

**Protocol Version**: 2025-06-18  
**Feature Status**: NEW - Added in this revision

Structured output enables tools to return validated JSON data alongside unstructured content, providing type safety and better integration with programming languages.

## Core Concepts

### Output Schema Definition
```typescript
interface Tool {
  name: string;
  description: string;
  inputSchema: JSONSchema;
  outputSchema?: JSONSchema;  // NEW - Optional output validation
}
```

### Structured Output in Tool Results
```typescript
interface CallToolResult {
  content: ContentBlock[];              // Traditional unstructured content
  structuredContent?: object;           // NEW - Validated JSON object
  isError?: boolean;
}
```

## Tool Definition with Output Schema

### Basic Weather Tool
```typescript
const weatherTool: Tool = {
  name: "get_weather_data",
  title: "Weather Data Retriever", 
  description: "Get current weather data for a location",
  inputSchema: {
    type: "object",
    properties: {
      location: {
        type: "string",
        description: "City name or zip code"
      }
    },
    required: ["location"]
  },
  outputSchema: {
    type: "object",
    properties: {
      temperature: {
        type: "number",
        description: "Temperature in celsius"
      },
      conditions: {
        type: "string",
        description: "Weather conditions description"
      },
      humidity: {
        type: "number",
        minimum: 0,
        maximum: 100,
        description: "Humidity percentage"
      },
      windSpeed: {
        type: "number",
        minimum: 0,
        description: "Wind speed in km/h"
      }
    },
    required: ["temperature", "conditions", "humidity"]
  }
};
```

### Complex Data Processing Tool
```typescript
const dataAnalysisTool: Tool = {
  name: "analyze_dataset",
  description: "Perform statistical analysis on a dataset",
  inputSchema: {
    type: "object", 
    properties: {
      datasetUrl: { type: "string", format: "uri" },
      analysisType: { 
        type: "string",
        enum: ["descriptive", "correlation", "regression"]
      }
    },
    required: ["datasetUrl", "analysisType"]
  },
  outputSchema: {
    type: "object",
    properties: {
      summary: {
        type: "object",
        properties: {
          totalRows: { type: "integer", minimum: 0 },
          totalColumns: { type: "integer", minimum: 0 },
          missingValues: { type: "integer", minimum: 0 }
        },
        required: ["totalRows", "totalColumns", "missingValues"]
      },
      statistics: {
        type: "object",
        properties: {
          mean: { type: "number" },
          median: { type: "number" },
          standardDeviation: { type: "number", minimum: 0 }
        }
      },
      correlations: {
        type: "array",
        items: {
          type: "object",
          properties: {
            variable1: { type: "string" },
            variable2: { type: "string" },
            coefficient: { type: "number", minimum: -1, maximum: 1 }
          },
          required: ["variable1", "variable2", "coefficient"]
        }
      }
    },
    required: ["summary"]
  }
};
```

## Implementation Patterns

### Server-Side Implementation
```typescript
async function executeWeatherTool(args: any): Promise<CallToolResult> {
  const { location } = args;
  
  try {
    // Fetch weather data from external API
    const weatherData = await fetchWeatherData(location);
    
    // Structure the data according to output schema
    const structuredData = {
      temperature: weatherData.temp_c,
      conditions: weatherData.condition.text,
      humidity: weatherData.humidity,
      windSpeed: weatherData.wind_kph
    };
    
    // Validate against output schema (recommended)
    const validation = validateOutputSchema(structuredData, weatherTool.outputSchema);
    if (!validation.valid) {
      throw new Error(`Output validation failed: ${validation.errors.join(', ')}`);
    }
    
    return {
      content: [
        {
          type: "text",
          text: `Weather in ${location}: ${structuredData.temperature}°C, ${structuredData.conditions}, ${structuredData.humidity}% humidity`,
          annotations: {
            audience: ["user"],
            priority: 0.9
          }
        }
      ],
      structuredContent: structuredData,
      isError: false
    };
    
  } catch (error) {
    return {
      content: [
        {
          type: "text",
          text: `Failed to fetch weather data: ${error.message}`
        }
      ],
      isError: true
    };
  }
}
```

### Output Schema Validation
```typescript
import Ajv from 'ajv';

function validateOutputSchema(data: any, schema: JSONSchema): ValidationResult {
  const ajv = new Ajv({ allErrors: true });
  const validate = ajv.compile(schema);
  
  if (!validate(data)) {
    return {
      valid: false,
      errors: validate.errors?.map(err => 
        `${err.instancePath}: ${err.message}`
      ) || []
    };
  }
  
  return { valid: true };
}

// Enhanced tool execution with validation
async function executeValidatedTool(
  toolName: string,
  args: any,
  tool: Tool
): Promise<CallToolResult> {
  
  const result = await executeToolFunction(toolName, args);
  
  // Validate structured output if schema provided
  if (tool.outputSchema && result.structuredContent) {
    const validation = validateOutputSchema(result.structuredContent, tool.outputSchema);
    
    if (!validation.valid) {
      // Log validation error but don't fail the request
      console.error(`Output schema validation failed for ${toolName}:`, validation.errors);
      
      // Optionally remove invalid structured content
      return {
        ...result,
        structuredContent: undefined,
        content: [
          ...result.content,
          {
            type: "text",
            text: "Warning: Tool output did not match expected schema"
          }
        ]
      };
    }
  }
  
  return result;
}
```

## Client-Side Usage

### Type-Safe Processing
```typescript
// Client can generate TypeScript types from output schema
interface WeatherData {
  temperature: number;
  conditions: string;
  humidity: number;
  windSpeed?: number;
}

async function callWeatherTool(location: string): Promise<WeatherData | null> {
  const response = await mcpClient.callTool("get_weather_data", { location });
  
  if (response.isError || !response.structuredContent) {
    console.error("Weather tool failed:", response.content[0]?.text);
    return null;
  }
  
  // Type-safe access to structured data
  const weatherData = response.structuredContent as WeatherData;
  
  // Validate data structure (runtime check)
  if (typeof weatherData.temperature !== 'number' || 
      typeof weatherData.conditions !== 'string') {
    console.error("Invalid weather data structure");
    return null;
  }
  
  return weatherData;
}
```

### Integration with Programming Languages
```typescript
// Generate code from structured output
async function generateWeatherReport(location: string): Promise<string> {
  const weatherData = await callWeatherTool(location);
  
  if (!weatherData) {
    return "Weather data unavailable";
  }
  
  // Use structured data for precise formatting
  return `
Location: ${location}
Temperature: ${weatherData.temperature}°C
Conditions: ${weatherData.conditions}
Humidity: ${weatherData.humidity}%
${weatherData.windSpeed ? `Wind Speed: ${weatherData.windSpeed} km/h` : ''}
  `.trim();
}
```

## Advanced Patterns

### Conditional Output Schemas
```typescript
const flexibleAnalysisTool: Tool = {
  name: "flexible_analysis",
  description: "Performs different types of analysis based on input",
  inputSchema: {
    type: "object",
    properties: {
      analysisType: {
        type: "string",
        enum: ["simple", "advanced", "custom"]
      },
      data: { type: "array" }
    },
    required: ["analysisType", "data"]
  },
  // Note: Complex conditional schemas can be defined using JSON Schema conditionals
  outputSchema: {
    type: "object",
    properties: {
      analysisType: { type: "string" },
      results: { type: "object" }
    },
    required: ["analysisType", "results"],
    allOf: [
      {
        if: { properties: { analysisType: { const: "simple" } } },
        then: {
          properties: {
            results: {
              type: "object",
              properties: {
                count: { type: "integer" },
                average: { type: "number" }
              },
              required: ["count", "average"]
            }
          }
        }
      },
      {
        if: { properties: { analysisType: { const: "advanced" } } },
        then: {
          properties: {
            results: {
              type: "object",
              properties: {
                statistics: { type: "object" },
                correlations: { type: "array" },
                predictions: { type: "array" }
              },
              required: ["statistics"]
            }
          }
        }
      }
    ]
  }
};
```

### Nested Data Structures
```typescript
const projectAnalysisTool: Tool = {
  name: "analyze_project_structure",
  outputSchema: {
    type: "object",
    properties: {
      project: {
        type: "object",
        properties: {
          name: { type: "string" },
          version: { type: "string" },
          languages: {
            type: "array",
            items: {
              type: "object",
              properties: {
                name: { type: "string" },
                percentage: { type: "number", minimum: 0, maximum: 100 },
                fileCount: { type: "integer", minimum: 0 }
              },
              required: ["name", "percentage", "fileCount"]
            }
          },
          dependencies: {
            type: "object",
            properties: {
              direct: { type: "array", items: { type: "string" } },
              transitive: { type: "array", items: { type: "string" } },
              vulnerable: { type: "array", items: { type: "string" } }
            },
            required: ["direct"]
          },
          metrics: {
            type: "object",
            properties: {
              linesOfCode: { type: "integer", minimum: 0 },
              complexity: { type: "number", minimum: 0 },
              testCoverage: { type: "number", minimum: 0, maximum: 100 }
            }
          }
        },
        required: ["name", "languages"]
      }
    },
    required: ["project"]
  }
};
```

## Benefits of Structured Output

### 1. Type Safety
- Compile-time checking in strongly-typed languages
- Runtime validation against schemas
- Reduced integration errors

### 2. Better Tooling
- IDE autocompletion and IntelliSense
- Automatic documentation generation
- Schema-driven code generation

### 3. Improved Integration
- Direct database insertion without parsing
- API consumption without manual validation
- Seamless data transformation pipelines

### 4. Enhanced Debugging
- Clear data structure expectations
- Validation error reporting
- Schema versioning support

## Migration Strategy

### Backwards Compatibility
```typescript
// Tools SHOULD provide both structured and unstructured content
async function executeCompatibleTool(args: any): Promise<CallToolResult> {
  const structuredData = await generateStructuredData(args);
  
  return {
    // Traditional text content for backward compatibility
    content: [
      {
        type: "text",
        text: JSON.stringify(structuredData, null, 2)
      }
    ],
    // NEW structured content for enhanced clients
    structuredContent: structuredData,
    isError: false
  };
}
```

### Schema Evolution
```typescript
// Version output schemas for evolution
const weatherToolV2: Tool = {
  name: "get_weather_data",
  outputSchema: {
    type: "object",
    properties: {
      // Existing fields (maintain compatibility)
      temperature: { type: "number" },
      conditions: { type: "string" },
      humidity: { type: "number" },
      
      // NEW fields (optional for compatibility)
      pressure: { type: "number" },
      uvIndex: { type: "integer", minimum: 0, maximum: 11 },
      forecast: {
        type: "array",
        items: {
          type: "object",
          properties: {
            date: { type: "string", format: "date" },
            temperature: { type: "number" },
            conditions: { type: "string" }
          }
        }
      }
    },
    required: ["temperature", "conditions", "humidity"] // Keep original requirements
  }
};
```