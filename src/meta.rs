//! Handles supplying custom metadata to the metrics scraper.
use std::{collections::HashMap, fmt::Debug, fs, path::Path, str::FromStr};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MetaFile {
    /// A list of fields to specify for each stream.
    pub fields: Vec<String>,
    /// The metadata to apply.
    pub metadata: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Clone, Default)]
pub struct MetaContainer {
    // A list of custom fields.
    fields: Vec<String>,
}

impl MetaContainer {
    /// Append a field to the metadata container.
    pub fn with_field<S: AsRef<str>>(mut self, field: S) -> Self {
        self.fields.push(field.as_ref().to_owned());
        self
    }
    /// Borrow and append a field to the metadata container.
    pub fn add_field<S: AsRef<str>>(&mut self, field: S) {
        self.fields.push(field.as_ref().to_owned());
    }
    /// Convert this container into a provider and append the value.
    pub fn with_value<S: AsRef<str>>(self, stream: S, field: S, value: S) -> Result<MetaProvider> {
        let provider = self.into_provider();
        provider.with_value(stream, field, value)
    }
    /// Convert this container into a provider.
    pub fn into_provider(self) -> MetaProvider {
        MetaProvider { fields: self.fields, metadata: HashMap::new() }
    }
    /// Clone this container and convert the cloned container into a provider.
    pub fn as_provider(&self) -> MetaProvider {
        self.clone().into_provider()
    }
}

#[derive(Default, Debug)]
pub struct MetaProvider {
    // A list of custom fields.
    fields: Vec<String>,
    // The stored metadata.
    metadata: HashMap<String, HashMap<String, String>>,
}

/// Enum for the supported formats of metadata file.
pub enum Format {
    /// The JSON format.
    JSON,
    /// The TOML format.
    TOML,
}

impl FromStr for Format {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Ok(Format::JSON),
            "toml" => Ok(Format::TOML),
            _ => bail!("Unknown format: {}", s),
        }
    }
}

impl MetaProvider {
    /// Create a metadata provider from a file, specifying the file format.
    pub fn from_file(path: impl AsRef<Path>, format: Format) -> Result<Self> {
        match format {
            Format::JSON => Self::from_json(path),
            Format::TOML => Self::from_toml(path),
        }
    }
    /// Create a metadata provider from a TOML file.
    pub fn from_toml<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = fs::read_to_string(path).context("Failed to read meta file")?;
        let file: MetaFile = toml::from_str(&file).context("Failed to parse meta file")?;
        Ok(Self::from_meta_file(file))
    }
    /// Create a metadata provider from a JSON file.
    pub fn from_json<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = fs::read_to_string(path).context("Failed to read meta file")?;
        let file: MetaFile = serde_json::from_str(&file).context("Failed to parse meta file")?;
        Ok(Self::from_meta_file(file))
    }
    /// Create a provider from a `MetaFile`.
    fn from_meta_file(file: MetaFile) -> Self {
        let mut container = MetaContainer::default();
        // iterate and add fields
        file.fields.iter().for_each(|field| container.add_field(field));
        // iterate and add values
        let mut container = container.as_provider();
        file.metadata
            .iter()
            .for_each(|(stream, meta)| container.add_value_many(stream, meta.iter()).unwrap());
        container
    }
    /// Add a value to this provider.
    pub fn with_value<S: AsRef<str>>(mut self, stream: S, field: S, value: S) -> Result<Self> {
        self.add_value(stream, field, value)?;
        Ok(self)
    }
    /// Borrow and add a value to this provider.
    pub fn add_value<S: AsRef<str>>(&mut self, stream: S, field: S, value: S) -> Result<()> {
        // check if an illegal field is specified
        if !self.fields.contains(&field.as_ref().to_owned()) {
            bail!("Unknown meta field: {}", field.as_ref());
        }
        // check if the stream is already in the metadata
        if !self.metadata.contains_key(stream.as_ref()) {
            self.metadata.insert(stream.as_ref().to_owned(), HashMap::default());
        }
        // get and insert
        let map = self.metadata.get_mut(stream.as_ref()).expect("Failed to get stream metadata");
        map.insert(field.as_ref().to_owned(), value.as_ref().to_owned());
        Ok(())
    }
    /// Borrow and add many values to the provider.
    pub fn add_value_many<S, I>(&mut self, stream: S, values: I) -> Result<()>
    where
        S: AsRef<str> + Clone + Debug,
        I: Iterator<Item = (S, S)> + Debug,
    {
        for (field, value) in values {
            self.add_value(stream.clone(), field, value)?;
        }
        Ok(())
    }

    /// Get a reference to the metadata.
    pub fn get_fields(&self) -> &Vec<String> {
        &self.fields
    }
    /// Get the field-sorted values of a stream's meta.
    pub fn get_values_for<S: AsRef<str> + Debug>(&self, stream: S) -> Vec<String> {
        self.fields
            .iter()
            .map(|field| {
                self.metadata
                    .get(stream.as_ref())
                    .unwrap_or(&HashMap::new())
                    .get(field)
                    .unwrap_or(&"unspecified".to_owned())
                    .to_owned()
            })
            .collect()
    }
    /// Return a vector containing all metadata entries.
    pub fn entries(&self) -> Vec<(String, String, String)> {
        self.metadata
            .iter()
            .flat_map(|(stream, meta)| {
                meta.iter().map(|(field, value)| (stream.clone(), field.clone(), value.clone()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::MetaFile;

    #[test]
    fn test_parse_meta_file_toml() {
        let file = r#"
fields = ["message"]

[metadata]
eaf8409c-6ee0-456b-aef8-d3477e6c5fdc = { message = "hello" }
"#;
        let file: MetaFile = toml::from_str(file).expect("failed to parse meta file");
        println!("{:?}", file);
        assert_eq!(file.fields, vec!["message"]);
        assert_eq!(file.metadata.len(), 1);
        assert_eq!(
            file.metadata
                .get("eaf8409c-6ee0-456b-aef8-d3477e6c5fdc")
                .expect("key did not exist")
                .len(),
            1
        );
        assert_eq!(
            file.metadata
                .get("eaf8409c-6ee0-456b-aef8-d3477e6c5fdc")
                .expect("key did not exist")
                .get("message")
                .expect("key did not exist"),
            "hello"
        );
    }

    #[test]
    fn test_parse_meta_file_json() {
        let file = r#"{
    "fields": ["message"],
	"metadata": {
		"eaf8409c-6ee0-456b-aef8-d3477e6c5fdc": {
			"message": "hello"
		}
	}
}"#;
        let file: MetaFile = serde_json::from_str(file).expect("failed to parse json");
        println!("{:?}", file);
        assert_eq!(file.fields, vec!["message"]);
        assert_eq!(file.metadata.len(), 1);
        assert_eq!(
            file.metadata
                .get("eaf8409c-6ee0-456b-aef8-d3477e6c5fdc")
                .expect("key did not exist")
                .len(),
            1
        );
        assert_eq!(
            file.metadata
                .get("eaf8409c-6ee0-456b-aef8-d3477e6c5fdc")
                .expect("key did not exist")
                .get("message")
                .expect("key did not exist"),
            "hello"
        );
    }
}
