use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub label: String,
    pub identifier: String,
    pub identifier_type: IdentifierType,
    pub address: String,
    pub notes: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IdentifierType {
    Email,
    Phone,
    Username,
}

impl IdentifierType {
    pub fn as_str(&self) -> &str {
        match self {
            IdentifierType::Email => "email",
            IdentifierType::Phone => "phone",
            IdentifierType::Username => "username",
        }
    }
}

pub struct AddressBook {
    db: Db,
}

impl AddressBook {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let db = sled::open(db_path).map_err(|e| format!("Failed to open database: {}", e))?;
        Ok(AddressBook { db })
    }

    /// Add a new contact
    pub fn add_contact(
        &self,
        label: String,
        identifier: String,
        identifier_type: IdentifierType,
        address: String,
        notes: Option<String>,
    ) -> Result<String, String> {
        // Generate unique ID
        let id = uuid::Uuid::new_v4().to_string();
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let contact = Contact {
            id: id.clone(),
            label,
            identifier: identifier.clone(),
            identifier_type,
            address,
            notes,
            created_at: timestamp,
        };

        // Serialize contact
        let contact_bytes = bincode::serialize(&contact)
            .map_err(|e| format!("Failed to serialize contact: {}", e))?;

        // Store by ID
        self.db
            .insert(format!("contact:{}", id), contact_bytes.as_slice())
            .map_err(|e| format!("Failed to store contact: {}", e))?;

        // Create index by identifier for fast lookup
        self.db
            .insert(format!("identifier:{}", identifier), id.as_bytes())
            .map_err(|e| format!("Failed to create identifier index: {}", e))?;

        self.db.flush().map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(id)
    }

    /// Find a contact by their identifier
    pub fn find_by_identifier(&self, identifier: &str) -> Result<Option<Contact>, String> {
        // Look up ID by identifier
        let id_key = format!("identifier:{}", identifier);
        if let Some(id_bytes) = self.db.get(&id_key).map_err(|e| e.to_string())? {
            let id = String::from_utf8(id_bytes.to_vec()).map_err(|e| e.to_string())?;
            
            // Get contact by ID
            let contact_key = format!("contact:{}", id);
            if let Some(contact_bytes) = self.db.get(&contact_key).map_err(|e| e.to_string())? {
                let contact: Contact = bincode::deserialize(&contact_bytes)
                    .map_err(|e| format!("Failed to deserialize contact: {}", e))?;
                return Ok(Some(contact));
            }
        }
        
        Ok(None)
    }

    /// Get all contacts
    pub fn get_all_contacts(&self) -> Result<Vec<Contact>, String> {
        let mut contacts = Vec::new();
        
        for item in self.db.scan_prefix("contact:") {
            let (_key, value) = item.map_err(|e| e.to_string())?;
            let contact: Contact = bincode::deserialize(&value)
                .map_err(|e| format!("Failed to deserialize contact: {}", e))?;
            contacts.push(contact);
        }
        
        // Sort by label
        contacts.sort_by(|a, b| a.label.cmp(&b.label));
        
        Ok(contacts)
    }

    /// Update a contact
    pub fn update_contact(
        &self,
        id: &str,
        label: String,
        identifier: String,
        identifier_type: IdentifierType,
        address: String,
        notes: Option<String>,
    ) -> Result<(), String> {
        // Get old contact to update identifier index
        let contact_key = format!("contact:{}", id);
        if let Some(old_bytes) = self.db.get(&contact_key).map_err(|e| e.to_string())? {
            let old_contact: Contact = bincode::deserialize(&old_bytes)
                .map_err(|e| format!("Failed to deserialize old contact: {}", e))?;
            
            // Remove old identifier index if it changed
            if old_contact.identifier != identifier {
                self.db.remove(format!("identifier:{}", old_contact.identifier))
                    .map_err(|e| e.to_string())?;
            }
        }

        let contact = Contact {
            id: id.to_string(),
            label,
            identifier: identifier.clone(),
            identifier_type,
            address,
            notes,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };

        let contact_bytes = bincode::serialize(&contact)
            .map_err(|e| format!("Failed to serialize contact: {}", e))?;

        self.db.insert(contact_key, contact_bytes.as_slice())
            .map_err(|e| e.to_string())?;

        self.db.insert(format!("identifier:{}", identifier), id.as_bytes())
            .map_err(|e| e.to_string())?;

        self.db.flush().map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// Delete a contact
    pub fn delete_contact(&self, id: &str) -> Result<(), String> {
        // Get contact to remove identifier index
        let contact_key = format!("contact:{}", id);
        if let Some(contact_bytes) = self.db.get(&contact_key).map_err(|e| e.to_string())? {
            let contact: Contact = bincode::deserialize(&contact_bytes)
                .map_err(|e| format!("Failed to deserialize contact: {}", e))?;
            
            // Remove identifier index
            self.db.remove(format!("identifier:{}", contact.identifier))
                .map_err(|e| e.to_string())?;
        }

        // Remove contact
        self.db.remove(contact_key).map_err(|e| e.to_string())?;
        self.db.flush().map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// Resolve an identifier to an address (key function for sending)
    pub fn resolve_address(&self, identifier_or_address: &str) -> String {
        // If it looks like an address already, return it as-is
        if identifier_or_address.len() == 64 || identifier_or_address.starts_with("tc1") {
            return identifier_or_address.to_string();
        }

        // Try to find it in the address book
        if let Ok(Some(contact)) = self.find_by_identifier(identifier_or_address) {
            return contact.address;
        }

        // If not found, return the original
        identifier_or_address.to_string()
    }
}
