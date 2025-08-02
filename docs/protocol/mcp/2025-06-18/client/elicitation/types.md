# Elicitation Request Types

**Protocol Revision**: 2025-06-18  
**🆕 NEW FEATURE** - Schema system for structured user input

Comprehensive reference for elicitation schema types, validation rules, and implementation patterns for collecting structured user data.

## Schema Architecture

### Core Schema Structure
```typescript
interface ElicitationSchema {
  type: "object";
  properties: {
    [propertyName: string]: PrimitiveSchemaDefinition;
  };
  required?: string[];
}
```

### Design Principles
- **Flat Structure**: Only top-level properties, no nesting
- **Primitive Types**: String, number, integer, boolean only
- **Client Simplicity**: Easy form generation and validation
- **Type Safety**: JSON Schema validation ensures data integrity

## Primitive Types

### String Schema

#### Basic String Type
```typescript
interface StringSchema {
  type: "string";
  title?: string;
  description?: string;
  minLength?: number;
  maxLength?: number;
  format?: "email" | "uri" | "date" | "date-time";
}
```

#### Examples
```json
{
  "username": {
    "type": "string",
    "title": "Username",
    "description": "Your account username",
    "minLength": 3,
    "maxLength": 20
  },
  "firstName": {
    "type": "string", 
    "title": "First Name",
    "minLength": 1,
    "maxLength": 50
  },
  "bio": {
    "type": "string",
    "title": "Biography",
    "description": "Brief description about yourself",
    "maxLength": 500
  }
}
```

#### Format Validation
```json
{
  "email": {
    "type": "string",
    "format": "email",
    "title": "Email Address",
    "description": "Your contact email"
  },
  "website": {
    "type": "string",
    "format": "uri", 
    "title": "Website URL",
    "description": "Your personal or company website"
  },
  "birthDate": {
    "type": "string",
    "format": "date",
    "title": "Birth Date",
    "description": "Format: YYYY-MM-DD"
  },
  "meetingTime": {
    "type": "string",
    "format": "date-time",
    "title": "Meeting Time", 
    "description": "Format: YYYY-MM-DDTHH:mm:ssZ"
  }
}
```

### Number Schema

#### Number vs Integer
```typescript
interface NumberSchema {
  type: "number" | "integer";
  title?: string;
  description?: string;
  minimum?: number;
  maximum?: number;
}
```

#### Examples
```json
{
  "age": {
    "type": "integer",
    "title": "Age", 
    "minimum": 13,
    "maximum": 120,
    "description": "Must be 13 or older"
  },
  "salary": {
    "type": "number",
    "title": "Annual Salary",
    "minimum": 0,
    "description": "In USD"
  },
  "score": {
    "type": "number",
    "title": "Test Score",
    "minimum": 0.0,
    "maximum": 100.0,
    "description": "Percentage score (0-100)"
  },
  "quantity": {
    "type": "integer",
    "title": "Quantity",
    "minimum": 1,
    "maximum": 99
  }
}
```

### Boolean Schema

#### Boolean Type Definition
```typescript
interface BooleanSchema {
  type: "boolean";
  title?: string;
  description?: string;
  default?: boolean;
}
```

#### Examples
```json
{
  "newsletter": {
    "type": "boolean",
    "title": "Subscribe to Newsletter",
    "description": "Receive weekly updates about new features",
    "default": false
  },
  "termsAccepted": {
    "type": "boolean",
    "title": "Accept Terms of Service",
    "description": "I agree to the terms and conditions"
  },
  "notifications": {
    "type": "boolean",
    "title": "Enable Notifications",
    "description": "Allow desktop notifications",
    "default": true
  }
}
```

### Enum Schema

#### Enum Type Definition
```typescript
interface EnumSchema {
  type: "string";
  title?: string;
  description?: string;
  enum: string[];
  enumNames?: string[];
}
```

#### Examples
```json
{
  "priority": {
    "type": "string",
    "title": "Priority Level",
    "enum": ["low", "medium", "high", "urgent"],
    "enumNames": ["Low", "Medium", "High", "Urgent"],
    "description": "Task priority level"
  },
  "theme": {
    "type": "string", 
    "title": "UI Theme",
    "enum": ["light", "dark", "auto"],
    "enumNames": ["Light Mode", "Dark Mode", "System Default"]
  },
  "language": {
    "type": "string",
    "title": "Programming Language",
    "enum": ["javascript", "typescript", "python", "rust", "go"],
    "enumNames": ["JavaScript", "TypeScript", "Python", "Rust", "Go"]
  }
}
```

## Complex Schema Examples

### User Registration Form
```json
{
  "message": "Complete your user registration",
  "requestedSchema": {
    "type": "object",
    "properties": {
      "username": {
        "type": "string",
        "title": "Username",
        "minLength": 3,
        "maxLength": 20,
        "description": "Letters, numbers, and underscores only"
      },
      "email": {
        "type": "string",
        "format": "email",
        "title": "Email Address"
      },
      "age": {
        "type": "integer",
        "title": "Age",
        "minimum": 13,
        "description": "Must be 13 or older to register"
      },
      "country": {
        "type": "string",
        "title": "Country",
        "enum": ["us", "ca", "uk", "de", "fr", "jp", "au"],
        "enumNames": ["United States", "Canada", "United Kingdom", "Germany", "France", "Japan", "Australia"]
      },
      "newsletter": {
        "type": "boolean",
        "title": "Subscribe to Newsletter",
        "default": false,
        "description": "Receive product updates and news"
      }
    },
    "required": ["username", "email", "age", "country"]
  }
}
```

### Project Configuration
```json
{
  "message": "Configure your new project settings",
  "requestedSchema": {
    "type": "object",
    "properties": {
      "projectName": {
        "type": "string",
        "title": "Project Name",
        "minLength": 1,
        "maxLength": 50,
        "description": "Name for your project"
      },
      "framework": {
        "type": "string",
        "title": "Framework",
        "enum": ["react", "vue", "angular", "svelte", "vanilla"],
        "enumNames": ["React", "Vue.js", "Angular", "Svelte", "Vanilla JS"]
      },
      "typescript": {
        "type": "boolean",
        "title": "Use TypeScript",
        "default": true,
        "description": "Enable TypeScript support"
      },
      "port": {
        "type": "integer",
        "title": "Development Port",
        "minimum": 1024,
        "maximum": 65535,
        "description": "Port for development server"
      },
      "apiUrl": {
        "type": "string",
        "format": "uri",
        "title": "API Base URL",
        "description": "Backend API endpoint"
      }
    },
    "required": ["projectName", "framework"]
  }
}
```

### Development Environment Setup
```json
{
  "message": "Set up your development environment preferences",
  "requestedSchema": {
    "type": "object",
    "properties": {
      "editor": {
        "type": "string",
        "title": "Code Editor",
        "enum": ["vscode", "vim", "emacs", "sublime", "atom", "webstorm"],
        "enumNames": ["VS Code", "Vim", "Emacs", "Sublime Text", "Atom", "WebStorm"]
      },
      "shellType": {
        "type": "string",
        "title": "Shell Type",
        "enum": ["bash", "zsh", "fish", "powershell"],
        "enumNames": ["Bash", "Zsh", "Fish", "PowerShell"]
      },
      "tabSize": {
        "type": "integer",
        "title": "Tab Size",
        "minimum": 2,
        "maximum": 8,
        "description": "Number of spaces for indentation"
      },
      "lineEndings": {
        "type": "string",
        "title": "Line Endings",
        "enum": ["lf", "crlf", "auto"],
        "enumNames": ["LF (Unix)", "CRLF (Windows)", "Auto-detect"]
      },
      "enableLinting": {
        "type": "boolean",
        "title": "Enable Linting",
        "default": true,
        "description": "Automatically lint code for errors"
      },
      "maxFileSize": {
        "type": "number",
        "title": "Max File Size (MB)",
        "minimum": 1,
        "maximum": 100,
        "description": "Maximum file size to process"
      }
    },
    "required": ["editor", "tabSize"]
  }
}
```

## Validation Rules

### String Validation
```typescript
function validateString(value: any, schema: StringSchema): ValidationResult {
  if (typeof value !== "string") {
    return { valid: false, error: "Value must be a string" };
  }
  
  if (schema.minLength && value.length < schema.minLength) {
    return { 
      valid: false, 
      error: `Minimum length is ${schema.minLength} characters` 
    };
  }
  
  if (schema.maxLength && value.length > schema.maxLength) {
    return { 
      valid: false, 
      error: `Maximum length is ${schema.maxLength} characters` 
    };
  }
  
  if (schema.format) {
    const formatValid = validateFormat(value, schema.format);
    if (!formatValid.valid) {
      return formatValid;
    }
  }
  
  return { valid: true };
}
```

### Number Validation
```typescript
function validateNumber(value: any, schema: NumberSchema): ValidationResult {
  const num = Number(value);
  if (isNaN(num)) {
    return { valid: false, error: "Value must be a number" };
  }
  
  if (schema.type === "integer" && !Number.isInteger(num)) {
    return { valid: false, error: "Value must be an integer" };
  }
  
  if (schema.minimum !== undefined && num < schema.minimum) {
    return { 
      valid: false, 
      error: `Value must be at least ${schema.minimum}` 
    };
  }
  
  if (schema.maximum !== undefined && num > schema.maximum) {
    return { 
      valid: false, 
      error: `Value must be at most ${schema.maximum}` 
    };
  }
  
  return { valid: true };
}
```

### Boolean Validation
```typescript
function validateBoolean(value: any, schema: BooleanSchema): ValidationResult {
  if (typeof value !== "boolean") {
    return { valid: false, error: "Value must be true or false" };
  }
  
  return { valid: true };
}
```

### Enum Validation
```typescript
function validateEnum(value: any, schema: EnumSchema): ValidationResult {
  if (typeof value !== "string") {
    return { valid: false, error: "Value must be a string" };
  }
  
  if (!schema.enum.includes(value)) {
    return { 
      valid: false, 
      error: `Value must be one of: ${schema.enum.join(', ')}` 
    };
  }
  
  return { valid: true };
}
```

### Format Validation
```typescript
function validateFormat(value: string, format: string): ValidationResult {
  switch (format) {
    case "email":
      const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
      return emailRegex.test(value) 
        ? { valid: true }
        : { valid: false, error: "Invalid email format" };
        
    case "uri":
      try {
        new URL(value);
        return { valid: true };
      } catch {
        return { valid: false, error: "Invalid URL format" };
      }
      
    case "date":
      const dateRegex = /^\d{4}-\d{2}-\d{2}$/;
      if (!dateRegex.test(value)) {
        return { valid: false, error: "Date must be in YYYY-MM-DD format" };
      }
      const date = new Date(value);
      return !isNaN(date.getTime())
        ? { valid: true }
        : { valid: false, error: "Invalid date" };
        
    case "date-time":
      const datetimeRegex = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{3})?Z?$/;
      if (!datetimeRegex.test(value)) {
        return { 
          valid: false, 
          error: "DateTime must be in ISO 8601 format" 
        };
      }
      const datetime = new Date(value);
      return !isNaN(datetime.getTime())
        ? { valid: true }
        : { valid: false, error: "Invalid datetime" };
        
    default:
      return { valid: true };
  }
}
```

## Schema Validation

### Complete Schema Validation
```typescript
function validateElicitationSchema(schema: any): ValidationResult {
  // Check root type
  if (schema.type !== "object") {
    return { valid: false, error: "Root schema must have type 'object'" };
  }
  
  // Check properties
  if (!schema.properties || typeof schema.properties !== "object") {
    return { valid: false, error: "Schema must have 'properties' object" };
  }
  
  // Validate each property
  for (const [key, propSchema] of Object.entries(schema.properties)) {
    const result = validatePropertySchema(key, propSchema);
    if (!result.valid) {
      return result;
    }
  }
  
  // Validate required array
  if (schema.required) {
    if (!Array.isArray(schema.required)) {
      return { valid: false, error: "'required' must be an array" };
    }
    
    for (const requiredProp of schema.required) {
      if (typeof requiredProp !== "string") {
        return { valid: false, error: "Required property names must be strings" };
      }
      
      if (!schema.properties[requiredProp]) {
        return { 
          valid: false, 
          error: `Required property '${requiredProp}' not found in properties` 
        };
      }
    }
  }
  
  return { valid: true };
}
```

### Property Schema Validation
```typescript
function validatePropertySchema(key: string, schema: any): ValidationResult {
  const validTypes = ["string", "number", "integer", "boolean"];
  
  if (!schema.type || !validTypes.includes(schema.type)) {
    return { 
      valid: false, 
      error: `Property '${key}' has invalid type. Must be one of: ${validTypes.join(', ')}` 
    };
  }
  
  // Type-specific validation
  switch (schema.type) {
    case "string":
      return validateStringSchema(key, schema);
    case "number":
    case "integer":
      return validateNumberSchema(key, schema);
    case "boolean":
      return validateBooleanSchema(key, schema);
    default:
      return { valid: true };
  }
}
```

## Client Form Generation

### Form Field Mapping
```typescript
interface FormField {
  type: "text" | "email" | "url" | "date" | "datetime" | "number" | "checkbox" | "select";
  name: string;
  label: string;
  description?: string;
  required: boolean;
  validation: ValidationRules;
  options?: SelectOption[];
  defaultValue?: any;
}

function generateFormField(name: string, schema: PrimitiveSchemaDefinition, required: boolean): FormField {
  const base = {
    name,
    label: schema.title || name,
    description: schema.description,
    required
  };
  
  switch (schema.type) {
    case "string":
      if (schema.enum) {
        return {
          ...base,
          type: "select",
          options: schema.enum.map((value, index) => ({
            value,
            label: schema.enumNames?.[index] || value
          })),
          validation: { enum: schema.enum }
        };
      }
      
      return {
        ...base,
        type: schema.format === "email" ? "email" 
            : schema.format === "uri" ? "url"
            : schema.format === "date" ? "date"
            : schema.format === "date-time" ? "datetime"
            : "text",
        validation: {
          minLength: schema.minLength,
          maxLength: schema.maxLength,
          format: schema.format
        }
      };
      
    case "number":
    case "integer":
      return {
        ...base,
        type: "number",
        validation: {
          minimum: schema.minimum,
          maximum: schema.maximum,
          integer: schema.type === "integer"
        }
      };
      
    case "boolean":
      return {
        ...base,
        type: "checkbox",
        defaultValue: schema.default,
        validation: {}
      };
      
    default:
      throw new Error(`Unsupported schema type: ${schema.type}`);
  }
}
```

## Error Messages

### User-Friendly Error Messages
```typescript
function formatValidationError(field: string, error: ValidationError): string {
  switch (error.type) {
    case "required":
      return `${field} is required`;
      
    case "minLength":
      return `${field} must be at least ${error.value} characters`;
      
    case "maxLength":
      return `${field} must be no more than ${error.value} characters`;
      
    case "minimum":
      return `${field} must be at least ${error.value}`;
      
    case "maximum":
      return `${field} must be no more than ${error.value}`;
      
    case "format":
      return `${field} has invalid format`;
      
    case "enum":
      return `${field} must be one of: ${error.allowedValues.join(', ')}`;
      
    case "type":
      return `${field} must be a ${error.expectedType}`;
      
    default:
      return `${field} is invalid`;
  }
}
```

## Implementation Examples

### Server Schema Creation
```typescript
class ConfigurationServer {
  createUserPreferencesSchema(): ElicitationSchema {
    return {
      type: "object",
      properties: {
        theme: {
          type: "string",
          title: "UI Theme",
          enum: ["light", "dark", "auto"],
          enumNames: ["Light", "Dark", "System Default"]
        },
        notifications: {
          type: "boolean",
          title: "Enable Notifications",
          default: true,
          description: "Receive desktop notifications"
        },
        refreshInterval: {
          type: "integer",
          title: "Refresh Interval (seconds)",
          minimum: 30,
          maximum: 3600,
          description: "How often to refresh data"
        },
        defaultEditor: {
          type: "string",
          title: "Default Code Editor",
          enum: ["vscode", "vim", "emacs"],
          enumNames: ["VS Code", "Vim", "Emacs"]
        }
      },
      required: ["theme", "defaultEditor"]
    };
  }
}
```

### Client Validation Implementation
```typescript
class ElicitationValidator {
  validateUserInput(data: any, schema: ElicitationSchema): ValidationResult {
    const errors: string[] = [];
    
    // Check required fields
    for (const requiredField of schema.required || []) {
      if (!(requiredField in data) || data[requiredField] === undefined || data[requiredField] === "") {
        errors.push(`${requiredField} is required`);
      }
    }
    
    // Validate each provided field
    for (const [field, value] of Object.entries(data)) {
      const fieldSchema = schema.properties[field];
      if (!fieldSchema) continue;
      
      const result = this.validateField(field, value, fieldSchema);
      if (!result.valid) {
        errors.push(result.error);
      }
    }
    
    return {
      valid: errors.length === 0,
      errors
    };
  }
}
```

## Related Documentation

- [Elicitation Overview](README.md) - High-level elicitation concepts
- [UI Requirements](ui-requirements.md) - Client implementation guidelines
- [Sampling](../sampling/) - AI model interaction features
- [Core Protocol](../../core/) - MCP protocol foundations