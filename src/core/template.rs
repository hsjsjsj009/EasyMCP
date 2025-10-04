use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write;
use tinytemplate::TinyTemplate;
use tinytemplate::error::Error;

pub struct Template {
    template: TinyTemplate<'static>,
    templates: HashMap<String, String>,
}

// Manually implement Send and Sync for Template
unsafe impl Send for Template {}
unsafe impl Sync for Template {}

impl Clone for Template {
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

impl Template {
    fn default_formatter(value: &Value, output: &mut String) -> Result<(), Error> {
        let object_string = serde_json::to_string(value)?;
        let object_string = object_string.trim_end_matches('"').trim_start_matches('"');
        output.write_str(&object_string)?;
        Ok(())
    }

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

        // Store the owned strings
        self.templates
            .insert(name_owned.clone(), template_owned.clone());

        // Use leaked strings for the TinyTemplate (this is safe for our use case)
        let name_static: &'static str = Box::leak(name_owned.into_boxed_str());
        let template_static: &'static str = Box::leak(template_owned.into_boxed_str());

        self.template.add_template(name_static, template_static)
    }

    pub fn render(&self, name: &str, input: &Value) -> Result<String, Error> {
        self.template.render(name, input)
    }
}
