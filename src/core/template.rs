use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write;
use tinytemplate::TinyTemplate;
use tinytemplate::error::Error;

pub struct Template<'a> {
    template: TinyTemplate<'a>,
    templates: HashMap<&'a str, &'a str>,
}

// Manually implement Send and Sync for Template
unsafe impl<'a> Send for Template<'a> {}
unsafe impl<'a> Sync for Template<'a> {}

impl<'a> Clone for Template<'a> {
    fn clone(&self) -> Self {
        let mut new_template = Template::new();
        // Re-register all templates from the original
        for (name, template_str) in &self.templates {
            new_template
                .add_template(name, template_str)
                .expect("Failed to clone template");
        }
        new_template
    }
}

impl<'a> Template<'a> {
    /// Default formatter that converts JSON values to their string representation.
    ///
    /// This formatter serializes the JSON value to a string and removes
    /// surrounding quotes for clean output.
    ///
    /// # Arguments
    /// * `value` - The JSON value to format
    /// * `output` - The output string to write the formatted result to
    ///
    /// # Returns
    /// * `Ok(())` if formatting was successful
    /// * `Err(Error)` if JSON serialization failed
    fn default_formatter(value: &Value, output: &mut String) -> Result<(), Error> {
        let object_string = serde_json::to_string(value)?;
        let object_string = object_string.trim_end_matches('"').trim_start_matches('"');
        output.write_str(&object_string)?;
        Ok(())
    }

    /// URL-encodes a JSON value for safe use in URLs and query parameters.
    ///
    /// This formatter converts the JSON value to its string representation,
    /// removes surrounding quotes, and then URL-encodes the result using
    /// the `urlencoding` crate. This is useful for including data in HTTP
    /// requests that might contain special characters.
    ///
    /// # Arguments
    /// * `value` - The JSON value to encode
    /// * `output` - The output string to write the encoded result to
    ///
    /// # Returns
    /// * `Ok(())` if encoding was successful
    /// * `Err(Error)` if JSON serialization failed
    fn url_encode_formatter(value: &Value, output: &mut String) -> Result<(), Error> {
        let object_string = serde_json::to_string(value)?;
        let encode = urlencoding::encode(object_string.as_str());
        output.write_str(&encode)?;
        Ok(())
    }

    pub fn new() -> Self {
        let mut template = TinyTemplate::new();
        template.set_default_formatter(&Self::default_formatter);
        template.add_formatter("url_encode", &Self::url_encode_formatter);
        Self {
            template,
            templates: HashMap::new(),
        }
    }

    pub fn add_template(&mut self, name: &str, template_str: &str) -> Result<(), Error> {
        let name_owned = name.to_string();
        let template_owned = template_str.to_string();

        // Use leaked strings for the TinyTemplate and TemplateMap (this is safe for our use case)
        let name_leak = Box::leak::<'a>(name_owned.into_boxed_str());
        let template_leak = Box::leak::<'a>(template_owned.into_boxed_str());

        // Store the owned strings
        self.templates.insert(name_leak, template_leak);

        self.template.add_template(name_leak, template_leak)
    }

    pub fn render(&self, name: &str, input: &Value) -> Result<String, Error> {
        self.template.render(name, input)
    }
}
