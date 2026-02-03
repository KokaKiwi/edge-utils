use headers::{Header, HeaderName, HeaderValue};

static GENERATION: HeaderName = HeaderName::from_static("generation");

pub struct Generation(pub u16);

impl Header for Generation {
    fn name() -> &'static HeaderName {
        &GENERATION
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(headers::Error::invalid)?
            .to_str()
            .map_err(|_| headers::Error::invalid())?;

        let generation = value
            .parse::<u16>()
            .map_err(|_| headers::Error::invalid())?;

        Ok(Generation(generation))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let value = HeaderValue::from_str(&self.0.to_string()).unwrap();
        values.extend(std::iter::once(value));
    }
}
