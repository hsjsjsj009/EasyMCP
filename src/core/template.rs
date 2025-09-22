use std::fmt::Write;
use serde_json::Value;
use tinytemplate::error::Error;
use tinytemplate::TinyTemplate;

fn default_formatter(value: &Value, output: &mut String) -> Result<(), Error> {
    let object_string = serde_json::to_string(value)?;
    output.write_str(&object_string)?;
    Ok(())
}

fn url_encode_formatter(value: &Value, output: &mut String) -> Result<(), Error> {
    let object_string = serde_json::to_string(value)?;
    let encode = urlencoding::encode(object_string.as_str());
    output.write_str(&encode)?;
    Ok(())
}

struct Template<'template> {
    template: TinyTemplate<'template>
}

impl<'template> Template<'template> {
    fn new() -> Self {
        let mut template = TinyTemplate::new();
        template.set_default_formatter(&default_formatter);
        template.add_formatter("url_encode", &url_encode_formatter);
        Self {
            template
        }
    }

    fn add_template(&mut self, name: &'template str, template: &'template str) -> Result<(), Error> {
        self.template.add_template(name, template)
    }

    fn render(&self, name: &'template str, input: &Value) -> Result<String, Error> {
        self.template.render(name, input)
    }
}
