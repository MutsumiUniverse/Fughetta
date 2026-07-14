pub trait EscapeGTKString {
    fn escape_string(&self) -> String;
}

impl EscapeGTKString for String {
    fn escape_string(&self) -> String {
        self.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
}
