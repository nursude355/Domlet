use wasm_bindgen::JsValue;
use web_sys::Element;

/// A small SVG line chart for telemetry-style browser UIs.
///
/// It has no animation loop, canvas runtime, or JavaScript dependency. Call
/// [`set_points`](Self::set_points) whenever the application state changes.
pub struct LineChart {
    svg: Element,
    line: Element,
}

impl LineChart {
    pub fn mount(host: &Element) -> Result<Self, JsValue> {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
        let namespace = Some("http://www.w3.org/2000/svg");
        let svg = document.create_element_ns(namespace, "svg")?;
        svg.set_attribute("viewBox", "0 0 100 100")?;
        svg.set_attribute("preserveAspectRatio", "none")?;
        svg.set_attribute("role", "img")?;
        svg.set_attribute("aria-label", "Live values")?;
        svg.set_attribute("width", "100%")?;
        svg.set_attribute("height", "100%")?;
        let line = document.create_element_ns(namespace, "polyline")?;
        line.set_attribute("fill", "none")?;
        line.set_attribute("stroke", "currentColor")?;
        line.set_attribute("stroke-width", "2")?;
        svg.append_child(&line)?;
        host.append_child(&svg)?;
        Ok(Self { svg, line })
    }

    /// Replaces the line with finite values, normalized to its own range.
    pub fn set_points(&self, values: &[f64]) -> Result<(), JsValue> {
        let points: Vec<f64> = values
            .iter()
            .copied()
            .filter(|value| value.is_finite())
            .collect();
        if points.len() < 2 {
            return self.line.set_attribute("points", "");
        }
        let low = points.iter().copied().fold(f64::INFINITY, f64::min);
        let high = points.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let span = (high - low).max(f64::EPSILON);
        let denominator = (points.len() - 1) as f64;
        let encoded = points
            .iter()
            .enumerate()
            .map(|(index, value)| {
                format!(
                    "{:.3},{:.3}",
                    index as f64 * 100.0 / denominator,
                    100.0 - (value - low) * 100.0 / span
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        self.line.set_attribute("points", &encoded)
    }
}

impl Drop for LineChart {
    fn drop(&mut self) {
        self.svg.remove();
    }
}
