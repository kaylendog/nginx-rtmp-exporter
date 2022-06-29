//! Handles supplying custom metadata to the metrics scraper.
use std::{collections::HashMap, fs, path::Path};

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

impl MetaProvider {
    /// Create a metadata provider from a TOML file.
    pub fn from_toml<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = fs::read_to_string(path).context("Failed to read meta file")?;
        let metafile: MetaFile = toml::from_str(&file).context("Failed to parse meta file")?;
        let mut container = MetaContainer::default();
        // iterate and add fields
        metafile.fields.iter().for_each(|field| container.add_field(field));
        // iterate and add values
        let mut container = container.as_provider();
        metafile
            .metadata
            .iter()
            .for_each(|(stream, meta)| container.add_value_many(stream, meta.iter()).unwrap());

        Ok(container)
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
        map.insert(field.as_ref().to_owned(), stream.as_ref().to_owned());
        Ok(())
    }
    /// Borrow and add many values to the provider.
    pub fn add_value_many<S, I>(&mut self, stream: S, values: I) -> Result<()>
    where
        S: AsRef<str> + Clone,
        I: Iterator<Item = (S, S)>,
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
}

#[cfg(test)]
mod tests {
    use super::MetaFile;

    #[test]
    fn test_parse_meta_file() {
        let file = r#"
fields = ["stadium"]

[metadata]
[[eaf8409c-6ee0-456b-aef8-d3477e6c5fdc]]
stadium = "hello"
"#;
        let _: MetaFile = toml::from_str(file).unwrap();
    }
}
