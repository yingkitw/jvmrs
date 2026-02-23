use byteorder::{BigEndian, ReadBytesExt};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Magic number for Java class files (0xCAFEBABE)
const CLASS_MAGIC: u32 = 0xCAFEBABE;

/// Represents a parsed Java class file
#[derive(Debug, Clone)]
pub struct ClassFile {
    pub magic: u32,
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool: Vec<ConstantPoolEntry>,
    pub access_flags: u16,
    pub this_class: u16,
    pub super_class: u16,
    pub interfaces: Vec<u16>,
    pub fields: Vec<FieldInfo>,
    pub methods: Vec<MethodInfo>,
    pub attributes: Vec<AttributeInfo>,
}

/// Constant pool entry types
#[derive(Debug, Clone)]
pub enum ConstantPoolEntry {
    ConstantClass {
        name_index: u16,
    },
    ConstantFieldref {
        class_index: u16,
        name_and_type_index: u16,
    },
    ConstantMethodref {
        class_index: u16,
        name_and_type_index: u16,
    },
    ConstantInterfaceMethodref {
        class_index: u16,
        name_and_type_index: u16,
    },
    ConstantString {
        string_index: u16,
    },
    ConstantInteger {
        bytes: i32,
    },
    ConstantFloat {
        bytes: f32,
    },
    ConstantLong {
        bytes: i64,
    },
    ConstantDouble {
        bytes: f64,
    },
    ConstantNameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    ConstantUtf8 {
        bytes: Vec<u8>,
    },
    ConstantMethodHandle {
        reference_kind: u8,
        reference_index: u16,
    },
    ConstantMethodType {
        descriptor_index: u16,
    },
    ConstantInvokeDynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
}

/// Field information
#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<AttributeInfo>,
}

/// Method information
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<AttributeInfo>,
}

/// Attribute information
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    pub attribute_name_index: u16,
    pub info: Vec<u8>,
}

/// Errors that can occur during class file parsing
#[derive(Debug)]
pub enum ParseError {
    IoError(io::Error),
    InvalidMagic(u32),
    InvalidConstantPoolTag(u8),
    InvalidAttributeLength,
}

impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> Self {
        ParseError::IoError(err)
    }
}

impl ClassFile {
    /// Parse a class file from the given path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ParseError> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Self::parse(&data)
    }

    /// Parse a class file from a byte slice
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let mut cursor = io::Cursor::new(data);

        // Read magic number
        let magic = cursor.read_u32::<BigEndian>()?;
        if magic != CLASS_MAGIC {
            return Err(ParseError::InvalidMagic(magic));
        }

        // Read version
        let minor_version = cursor.read_u16::<BigEndian>()?;
        let major_version = cursor.read_u16::<BigEndian>()?;

        // Read constant pool count
        let constant_pool_count = cursor.read_u16::<BigEndian>()?;
        let mut constant_pool = Vec::with_capacity(constant_pool_count as usize);
        constant_pool.push(ConstantPoolEntry::ConstantUtf8 { bytes: Vec::new() }); // Index 0 is invalid

        // Read constant pool entries
        for _ in 1..constant_pool_count {
            let tag = cursor.read_u8()?;
            let entry = Self::read_constant_pool_entry(&mut cursor, tag)?;
            constant_pool.push(entry);

            // Long and Double take two constant pool slots
            if matches!(tag, 5 | 6) {
                constant_pool.push(ConstantPoolEntry::ConstantUtf8 { bytes: Vec::new() });
            }
        }

        // Read access flags
        let access_flags = cursor.read_u16::<BigEndian>()?;

        // Read this class and super class
        let this_class = cursor.read_u16::<BigEndian>()?;
        let super_class = cursor.read_u16::<BigEndian>()?;

        // Read interfaces
        let interfaces_count = cursor.read_u16::<BigEndian>()?;
        let mut interfaces = Vec::with_capacity(interfaces_count as usize);
        for _ in 0..interfaces_count {
            interfaces.push(cursor.read_u16::<BigEndian>()?);
        }

        // Read fields
        let fields_count = cursor.read_u16::<BigEndian>()?;
        let mut fields = Vec::with_capacity(fields_count as usize);
        for _ in 0..fields_count {
            fields.push(Self::read_field_info(&mut cursor)?);
        }

        // Read methods
        let methods_count = cursor.read_u16::<BigEndian>()?;
        let mut methods = Vec::with_capacity(methods_count as usize);
        for _ in 0..methods_count {
            methods.push(Self::read_method_info(&mut cursor)?);
        }

        // Read attributes
        let attributes_count = cursor.read_u16::<BigEndian>()?;
        let mut attributes = Vec::with_capacity(attributes_count as usize);
        for _ in 0..attributes_count {
            attributes.push(Self::read_attribute_info(&mut cursor)?);
        }

        Ok(ClassFile {
            magic,
            minor_version,
            major_version,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            attributes,
        })
    }

    fn read_constant_pool_entry(
        cursor: &mut io::Cursor<&[u8]>,
        tag: u8,
    ) -> Result<ConstantPoolEntry, ParseError> {
        match tag {
            1 => {
                // CONSTANT_Utf8
                let len = cursor.read_u16::<BigEndian>()? as usize;
                let mut bytes = vec![0u8; len];
                cursor.read_exact(&mut bytes)?;
                Ok(ConstantPoolEntry::ConstantUtf8 { bytes })
            }
            3 => {
                // CONSTANT_Integer
                let bytes = cursor.read_i32::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantInteger { bytes })
            }
            4 => {
                // CONSTANT_Float
                let bytes = cursor.read_u32::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantFloat {
                    bytes: f32::from_bits(bytes),
                })
            }
            5 => {
                // CONSTANT_Long
                let bytes = cursor.read_i64::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantLong { bytes })
            }
            6 => {
                // CONSTANT_Double
                let bytes = cursor.read_u64::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantDouble {
                    bytes: f64::from_bits(bytes),
                })
            }
            7 => {
                // CONSTANT_Class
                let name_index = cursor.read_u16::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantClass { name_index })
            }
            8 => {
                // CONSTANT_String
                let string_index = cursor.read_u16::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantString { string_index })
            }
            9 => {
                // CONSTANT_Fieldref
                let class_index = cursor.read_u16::<BigEndian>()?;
                let name_and_type_index = cursor.read_u16::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantFieldref {
                    class_index,
                    name_and_type_index,
                })
            }
            10 => {
                // CONSTANT_Methodref
                let class_index = cursor.read_u16::<BigEndian>()?;
                let name_and_type_index = cursor.read_u16::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantMethodref {
                    class_index,
                    name_and_type_index,
                })
            }
            11 => {
                // CONSTANT_InterfaceMethodref
                let class_index = cursor.read_u16::<BigEndian>()?;
                let name_and_type_index = cursor.read_u16::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantInterfaceMethodref {
                    class_index,
                    name_and_type_index,
                })
            }
            12 => {
                // CONSTANT_NameAndType
                let name_index = cursor.read_u16::<BigEndian>()?;
                let descriptor_index = cursor.read_u16::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantNameAndType {
                    name_index,
                    descriptor_index,
                })
            }
            15 => {
                // CONSTANT_MethodHandle
                let reference_kind = cursor.read_u8()?;
                let reference_index = cursor.read_u16::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantMethodHandle {
                    reference_kind,
                    reference_index,
                })
            }
            16 => {
                // CONSTANT_MethodType
                let descriptor_index = cursor.read_u16::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantMethodType { descriptor_index })
            }
            18 => {
                // CONSTANT_InvokeDynamic
                let bootstrap_method_attr_index = cursor.read_u16::<BigEndian>()?;
                let name_and_type_index = cursor.read_u16::<BigEndian>()?;
                Ok(ConstantPoolEntry::ConstantInvokeDynamic {
                    bootstrap_method_attr_index,
                    name_and_type_index,
                })
            }
            _ => Err(ParseError::InvalidConstantPoolTag(tag)),
        }
    }

    fn read_field_info(cursor: &mut io::Cursor<&[u8]>) -> Result<FieldInfo, ParseError> {
        let access_flags = cursor.read_u16::<BigEndian>()?;
        let name_index = cursor.read_u16::<BigEndian>()?;
        let descriptor_index = cursor.read_u16::<BigEndian>()?;

        let attributes_count = cursor.read_u16::<BigEndian>()?;
        let mut attributes = Vec::with_capacity(attributes_count as usize);
        for _ in 0..attributes_count {
            attributes.push(Self::read_attribute_info(cursor)?);
        }

        Ok(FieldInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }

    fn read_method_info(cursor: &mut io::Cursor<&[u8]>) -> Result<MethodInfo, ParseError> {
        let access_flags = cursor.read_u16::<BigEndian>()?;
        let name_index = cursor.read_u16::<BigEndian>()?;
        let descriptor_index = cursor.read_u16::<BigEndian>()?;

        let attributes_count = cursor.read_u16::<BigEndian>()?;
        let mut attributes = Vec::with_capacity(attributes_count as usize);
        for _ in 0..attributes_count {
            attributes.push(Self::read_attribute_info(cursor)?);
        }

        Ok(MethodInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }

    fn read_attribute_info(cursor: &mut io::Cursor<&[u8]>) -> Result<AttributeInfo, ParseError> {
        let attribute_name_index = cursor.read_u16::<BigEndian>()?;
        let length = cursor.read_u32::<BigEndian>()? as usize;
        let mut info = vec![0u8; length];
        cursor.read_exact(&mut info)?;
        Ok(AttributeInfo {
            attribute_name_index,
            info,
        })
    }

    /// Get a string from the constant pool by index
    pub fn get_string(&self, index: u16) -> Option<String> {
        let entry = self.constant_pool.get(index as usize)?;
        match entry {
            ConstantPoolEntry::ConstantUtf8 { bytes } => String::from_utf8(bytes.clone()).ok(),
            _ => None,
        }
    }

    /// Get the class name
    pub fn get_class_name(&self) -> Option<String> {
        let class_entry = self.constant_pool.get(self.this_class as usize)?;
        match class_entry {
            ConstantPoolEntry::ConstantClass { name_index } => self.get_string(*name_index),
            _ => None,
        }
    }

    /// Get the super class name
    pub fn get_super_class_name(&self) -> Option<String> {
        if self.super_class == 0 {
            return None; // java/lang/Object has no super class
        }
        let class_entry = self.constant_pool.get(self.super_class as usize)?;
        match class_entry {
            ConstantPoolEntry::ConstantClass { name_index } => self.get_string(*name_index),
            _ => None,
        }
    }

    /// Find a method by name and descriptor
    pub fn find_method(&self, name: &str, descriptor: &str) -> Option<&MethodInfo> {
        self.methods.iter().find(|m| {
            let method_name = self.get_string(m.name_index).unwrap_or_default();
            let method_desc = self.get_string(m.descriptor_index).unwrap_or_default();
            method_name == name && method_desc == descriptor
        })
    }
}
