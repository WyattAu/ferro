pub mod ical;
pub mod store;
pub mod vcard;
pub mod xml_ext;

#[cfg(feature = "handlers")]
pub mod caldav;

#[cfg(feature = "handlers")]
pub mod carddav;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod xml_ext_proptest;

#[cfg(test)]
mod vcard_proptest;

#[cfg(test)]
mod ical_proptest;
