use crate::class_file::ClassFile;
use crate::memory::Value;

/// Reflection information about a field
#[derive(Debug, Clone)]
pub struct FieldReflection {
    pub name: String,
    pub field_type: String,
    pub modifiers: u16,
    pub value: Option<Value>,
}

/// Reflection information about a method
#[derive(Debug, Clone)]
pub struct MethodReflection {
    pub name: String,
    pub parameter_types: Vec<String>,
    pub return_type: String,
    pub modifiers: u16,
}

/// Reflection information about a constructor
#[derive(Debug, Clone)]
pub struct ConstructorReflection {
    pub parameter_types: Vec<String>,
    pub modifiers: u16,
}

/// Reflection information about a class
#[derive(Debug, Clone)]
pub struct ClassReflection {
    pub name: String,
    pub fields: Vec<FieldReflection>,
    pub methods: Vec<MethodReflection>,
    pub constructors: Vec<ConstructorReflection>,
    pub super_class: Option<String>,
    pub interfaces: Vec<String>,
    pub modifiers: u16,
}

/// Convert JVM descriptor to readable type name
fn descriptor_to_type(descriptor: &str) -> String {
    let desc = descriptor.trim();
    if desc.is_empty() {
        return "void".to_string();
    }
    if let Some(c) = desc.chars().next() {
        match c {
            'B' => return "byte".to_string(),
            'C' => return "char".to_string(),
            'D' => return "double".to_string(),
            'F' => return "float".to_string(),
            'I' => return "int".to_string(),
            'J' => return "long".to_string(),
            'S' => return "short".to_string(),
            'Z' => return "boolean".to_string(),
            'V' => return "void".to_string(),
            'L' => {
                if let Some(semi) = desc.find(';') {
                    return desc[1..semi].replace('/', ".");
                }
            }
            '[' => {
                if desc.len() > 1 {
                    return format!("[{}]", descriptor_to_type(&desc[1..]));
                }
            }
            _ => {}
        }
    }
    descriptor.to_string()
}

/// Convert ClassFile to ClassReflection (for use with loaded classes)
pub fn class_to_reflection(class: &ClassFile) -> ClassReflection {
    let fields = class
        .fields
        .iter()
        .map(|f| FieldReflection {
            name: class.get_string(f.name_index).unwrap_or_default(),
            field_type: descriptor_to_type(
                &class.get_string(f.descriptor_index).unwrap_or_default(),
            ),
            modifiers: f.access_flags,
            value: None,
        })
        .collect();

    let methods = class
        .methods
        .iter()
        .filter(|m| {
            let name = class.get_string(m.name_index).unwrap_or_default();
            name != "<init>" && name != "<clinit>"
        })
        .map(|m| {
            let desc = class.get_string(m.descriptor_index).unwrap_or_default();
            let params = parse_method_params(&desc);
            let return_type = parse_return_type(&desc);
            MethodReflection {
                name: class.get_string(m.name_index).unwrap_or_default(),
                parameter_types: params,
                return_type,
                modifiers: m.access_flags,
            }
        })
        .collect();

    let constructors = class
        .methods
        .iter()
        .filter(|m| class.get_string(m.name_index).as_deref() == Some("<init>"))
        .map(|m| {
            let desc = class.get_string(m.descriptor_index).unwrap_or_default();
            ConstructorReflection {
                parameter_types: parse_method_params(&desc),
                modifiers: m.access_flags,
            }
        })
        .collect();

    let interfaces = class
        .interfaces
        .iter()
        .filter_map(|i| class.get_string(*i))
        .collect();

    ClassReflection {
        name: class.get_class_name().unwrap_or_default(),
        fields,
        methods,
        constructors,
        super_class: class.get_super_class_name(),
        interfaces,
        modifiers: class.access_flags,
    }
}

fn parse_method_params(descriptor: &str) -> Vec<String> {
    let mut params = Vec::new();
    if let Some(start) = descriptor.find('(') {
        let rest = &descriptor[start + 1..];
        if let Some(end) = rest.find(')') {
            let params_str = &rest[..end];
            let mut i = 0;
            let chars: Vec<char> = params_str.chars().collect();
            while i < chars.len() {
                match chars[i] {
                    'B' => {
                        params.push("byte".to_string());
                        i += 1;
                    }
                    'C' => {
                        params.push("char".to_string());
                        i += 1;
                    }
                    'D' => {
                        params.push("double".to_string());
                        i += 1;
                    }
                    'F' => {
                        params.push("float".to_string());
                        i += 1;
                    }
                    'I' => {
                        params.push("int".to_string());
                        i += 1;
                    }
                    'J' => {
                        params.push("long".to_string());
                        i += 1;
                    }
                    'S' => {
                        params.push("short".to_string());
                        i += 1;
                    }
                    'Z' => {
                        params.push("boolean".to_string());
                        i += 1;
                    }
                    'L' => {
                        let mut j = i + 1;
                        while j < chars.len() && chars[j] != ';' {
                            j += 1;
                        }
                        let s: String = chars[i + 1..j].iter().collect();
                        params.push(s.replace('/', "."));
                        i = j + 1;
                    }
                    '[' => {
                        let mut depth = 1;
                        let mut j = i + 1;
                        while j < chars.len() && depth > 0 {
                            if chars[j] == '[' {
                                depth += 1;
                            } else if chars[j] == ']' {
                                depth -= 1;
                            }
                            j += 1;
                        }
                        params.push(chars[i..j].iter().collect::<String>());
                        i = j;
                    }
                    _ => i += 1,
                }
            }
        }
    }
    params
}

fn parse_return_type(descriptor: &str) -> String {
    if let Some(p) = descriptor.rfind(')') {
        descriptor_to_type(&descriptor[p + 1..])
    } else {
        "void".to_string()
    }
}

/// Reflection API for runtime introspection
pub struct ReflectionApi {}

impl ReflectionApi {
    /// Create a new reflection API instance
    pub fn new() -> Self {
        Self {}
    }

    /// Get class information for a given class name.
    /// Returns placeholder data for unloaded classes. For loaded classes,
    /// use `Interpreter::get_class_reflection` instead.
    pub fn get_class(&self, class_name: &str) -> Option<ClassReflection> {
        Some(ClassReflection {
            name: class_name.to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            constructors: Vec::new(),
            super_class: None,
            interfaces: Vec::new(),
            modifiers: 0,
        })
    }

    /// Get field information for a class
    pub fn get_fields(&self, class_name: &str) -> Vec<FieldReflection> {
        let class_reflection = self
            .get_class(class_name)
            .unwrap_or_else(|| ClassReflection {
                name: class_name.to_string(),
                fields: Vec::new(),
                methods: Vec::new(),
                constructors: Vec::new(),
                super_class: None,
                interfaces: Vec::new(),
                modifiers: 0,
            });
        class_reflection.fields.clone()
    }

    /// Get method information for a class
    pub fn get_methods(&self, class_name: &str) -> Vec<MethodReflection> {
        let class_reflection = self
            .get_class(class_name)
            .unwrap_or_else(|| ClassReflection {
                name: class_name.to_string(),
                fields: Vec::new(),
                methods: Vec::new(),
                constructors: Vec::new(),
                super_class: None,
                interfaces: Vec::new(),
                modifiers: 0,
            });
        class_reflection.methods.clone()
    }

    /// Get constructor information for a class
    pub fn get_constructors(&self, class_name: &str) -> Vec<ConstructorReflection> {
        let class_reflection = self
            .get_class(class_name)
            .unwrap_or_else(|| ClassReflection {
                name: class_name.to_string(),
                fields: Vec::new(),
                methods: Vec::new(),
                constructors: Vec::new(),
                super_class: None,
                interfaces: Vec::new(),
                modifiers: 0,
            });
        class_reflection.constructors.clone()
    }

    /// Get the super class of a class
    pub fn get_super_class(&self, class_name: &str) -> Option<String> {
        let class_reflection = self
            .get_class(class_name)
            .unwrap_or_else(|| ClassReflection {
                name: class_name.to_string(),
                fields: Vec::new(),
                methods: Vec::new(),
                constructors: Vec::new(),
                super_class: None,
                interfaces: Vec::new(),
                modifiers: 0,
            });
        class_reflection.super_class.clone()
    }

    /// Get the interfaces implemented by a class
    pub fn get_interfaces(&self, class_name: &str) -> Vec<String> {
        let class_reflection = self
            .get_class(class_name)
            .unwrap_or_else(|| ClassReflection {
                name: class_name.to_string(),
                fields: Vec::new(),
                methods: Vec::new(),
                constructors: Vec::new(),
                super_class: None,
                interfaces: Vec::new(),
                modifiers: 0,
            });
        class_reflection.interfaces.clone()
    }

    /// Get the modifiers of a class
    pub fn get_class_modifiers(&self, class_name: &str) -> u16 {
        let class_reflection = self
            .get_class(class_name)
            .unwrap_or_else(|| ClassReflection {
                name: class_name.to_string(),
                fields: Vec::new(),
                methods: Vec::new(),
                constructors: Vec::new(),
                super_class: None,
                interfaces: Vec::new(),
                modifiers: 0,
            });
        class_reflection.modifiers
    }

    /// Check if a class is an interface
    pub fn is_interface(&self, class_name: &str) -> bool {
        let modifiers = self.get_class_modifiers(class_name);
        (modifiers & 0x0200) != 0 // ACC_INTERFACE
    }

    /// Check if a class is abstract
    pub fn is_abstract(&self, class_name: &str) -> bool {
        let modifiers = self.get_class_modifiers(class_name);
        (modifiers & 0x0400) != 0 // ACC_ABSTRACT
    }

    /// Check if a class is public
    pub fn is_public(&self, class_name: &str) -> bool {
        let modifiers = self.get_class_modifiers(class_name);
        (modifiers & 0x0001) != 0 // ACC_PUBLIC
    }

    /// Check if a field is public
    pub fn is_field_public(&self, class_name: &str, field_name: &str) -> bool {
        let fields = self.get_fields(class_name);
        for field in fields {
            if field.name == field_name {
                return (field.modifiers & 0x0001) != 0; // ACC_PUBLIC
            }
        }
        false
    }

    /// Check if a field is static
    pub fn is_field_static(&self, class_name: &str, field_name: &str) -> bool {
        let fields = self.get_fields(class_name);
        for field in fields {
            if field.name == field_name {
                return (field.modifiers & 0x0008) != 0; // ACC_STATIC
            }
        }
        false
    }

    /// Check if a method is public
    pub fn is_method_public(&self, class_name: &str, method_name: &str) -> bool {
        let methods = self.get_methods(class_name);
        for method in methods {
            if method.name == method_name {
                return (method.modifiers & 0x0001) != 0; // ACC_PUBLIC
            }
        }
        false
    }

    /// Check if a method is static
    pub fn is_method_static(&self, class_name: &str, method_name: &str) -> bool {
        let methods = self.get_methods(class_name);
        for method in methods {
            if method.name == method_name {
                return (method.modifiers & 0x0008) != 0; // ACC_STATIC
            }
        }
        false
    }

    /// Create a new instance of a class using reflection
    pub fn new_instance(&self, class_name: &str, _args: &[Value]) -> Result<Value, String> {
        // This is a simplified implementation - just return a placeholder object reference
        // In a real JVM, this would find the appropriate constructor and invoke it
        Ok(Value::Reference(12345)) // Placeholder address
    }

    /// Get the value of a field from an object
    pub fn get_field_value(&self, obj: &Value, field_name: &str) -> Result<Value, String> {
        match obj {
            Value::Reference(_addr) => {
                // Simplified implementation - return a placeholder value
                Ok(Value::Int(42))
            }
            _ => Err("Not an object reference".to_string()),
        }
    }

    /// Set the value of a field in an object
    pub fn set_field_value(
        &self,
        obj: &Value,
        field_name: &str,
        value: Value,
    ) -> Result<(), String> {
        match obj {
            Value::Reference(_addr) => {
                // Simplified implementation - just return success
                Ok(())
            }
            _ => Err("Not an object reference".to_string()),
        }
    }

    /// Invoke a method using reflection
    pub fn invoke_method(
        &self,
        obj: &Value,
        method_name: &str,
        _args: &[Value],
    ) -> Result<Value, String> {
        match obj {
            Value::Reference(_addr) => {
                // Simplified implementation - just return a placeholder value
                Ok(Value::Int(42))
            }
            _ => Err("Not an object reference".to_string()),
        }
    }

    /// Get the class of an object
    pub fn get_object_class(&self, obj: &Value) -> Result<String, String> {
        match obj {
            Value::Reference(_addr) => {
                // Simplified implementation - return a placeholder class name
                Ok("java/lang/Object".to_string())
            }
            _ => Err("Not an object reference".to_string()),
        }
    }
}

/// Access flags for fields and methods
pub mod modifiers {
    pub const PUBLIC: u16 = 0x0001;
    pub const PRIVATE: u16 = 0x0002;
    pub const PROTECTED: u16 = 0x0004;
    pub const STATIC: u16 = 0x0008;
    pub const FINAL: u16 = 0x0010;
    pub const SYNCHRONIZED: u16 = 0x0020;
    pub const VOLATILE: u16 = 0x0040;
    pub const TRANSIENT: u16 = 0x0080;
    pub const NATIVE: u16 = 0x0100;
    pub const INTERFACE: u16 = 0x0200;
    pub const ABSTRACT: u16 = 0x0400;
    pub const STRICTFP: u16 = 0x0800;
}

impl Default for ReflectionApi {
    fn default() -> Self {
        Self::new()
    }
}
