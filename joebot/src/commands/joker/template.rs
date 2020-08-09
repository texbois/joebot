use crate::JoeResult;
use std::process::Command;

pub struct Template {
    rel_path: String,
    size_str: String,
    width: u32,
}

pub fn load_jpg_templates(dir: &str) -> JoeResult<Vec<Template>> {
    let mut templates = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().map(|s| s == "jpg").unwrap_or(false) {
            let rel_path = format!("{}/{}", dir, path.file_name().unwrap().to_str().unwrap());
            templates.push(Template::new(rel_path)?);
        }
    }
    Ok(templates)
}

impl Template {
    fn new(rel_path: String) -> JoeResult<Self> {
        let size_raw = Command::new("identify")
            .args(&["-format", "%wx%h", &rel_path])
            .output()?
            .stdout;
        let size_str = String::from_utf8(size_raw).unwrap();
        let mut size_tup = size_str.split('x');
        let width = size_tup.next().unwrap().parse::<u32>().unwrap();
        //let height = size_tup.next().unwrap().parse::<u32>().unwrap();

        Ok(Self {
            rel_path,
            size_str,
            width,
        })
    }

    fn font_size(&self, text_len: usize) -> usize {
        let size_k = if text_len <= 100 { 0.0427 } else { 0.0333 };
        (size_k * self.width as f64) as usize
    }

    pub fn render(&self, top: &str, bottom: &str, font: &str) -> JoeResult<Vec<u8>> {
        let top_caption = format!("caption:{}", top);
        let bottom_caption = format!("caption:{}", bottom);
        let max_text_len = std::cmp::max(top.chars().count(), bottom.chars().count());
        let font_size = self.font_size(max_text_len).to_string();

        let rendered = Command::new("convert")
            .args(&[
                &self.rel_path,
                "-font",
                font,
                "-pointsize",
                &font_size,
                "-fill",
                "white",
                "-stroke",
                "black",
                "-strokewidth",
                "8",
            ])
            .args(&[
                "(",
                "-size",
                &self.size_str,
                "-background",
                "transparent",
                "-gravity",
                "north",
                &top_caption,
                "-stroke",
                "none",
                &top_caption,
                "-repage",
                "+0+20",
                ")",
            ])
            .args(&[
                "-stroke",
                "black",
                "(",
                "-size",
                &self.size_str,
                "-background",
                "transparent",
                "-gravity",
                "south",
                &bottom_caption,
                "-stroke",
                "none",
                &bottom_caption,
                "-repage",
                "+0-30",
                ")",
                "-flatten",
                "jpeg:-",
            ])
            .output()?
            .stdout;

        Ok(rendered)
    }
}
