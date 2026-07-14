use proptest::prelude::*;

use crate::ical::{IcalComponent, IcalProperty, get_all_props, get_first_prop, parse_ical, serialize_ical};
use hashbrown::HashMap;

fn arb_ical_property() -> impl Strategy<Value = IcalProperty> {
    ("[A-Z]{2,15}", "[a-zA-Z0-9:,; .-]{0,100}").prop_map(|(name, value)| IcalProperty {
        name,
        params: HashMap::new(),
        value,
    })
}

fn arb_ical_component_simple() -> impl Strategy<Value = IcalComponent> {
    prop::collection::vec(arb_ical_property(), 0..5).prop_map(|properties| {
        let mut props_map = HashMap::new();
        for prop in properties {
            props_map.entry(prop.name.clone()).or_insert_with(Vec::new).push(prop);
        }
        IcalComponent {
            name: "VEVENT".to_string(),
            properties: props_map,
            children: vec![],
        }
    })
}

proptest! {
    #[test]
    fn ical_roundtrip_simple(component in arb_ical_component_simple()) {
        let wrapped = IcalComponent {
            name: "VCALENDAR".to_string(),
            properties: HashMap::new(),
            children: vec![component.clone()],
        };
        let serialized = serialize_ical(&[wrapped]);
        let parsed = parse_ical(&serialized).unwrap();

        prop_assert!(!parsed.is_empty());
        let cal = &parsed[0];
        prop_assert_eq!(&cal.name, "VCALENDAR");
        prop_assert_eq!(cal.children.len(), 1);

        let child = &cal.children[0];
        prop_assert_eq!(&child.name, &component.name);
    }

    #[test]
    fn ical_serialize_never_panics(component in arb_ical_component_simple()) {
        let _ = serialize_ical(&[IcalComponent {
            name: "VCALENDAR".to_string(),
            properties: HashMap::new(),
            children: vec![component],
        }]);
    }

    #[test]
    fn ical_parse_never_panics(input in ".*") {
        let _ = parse_ical(&input);
    }

    #[test]
    fn ical_has_begin_end(component in arb_ical_component_simple()) {
        let wrapped = IcalComponent {
            name: "VCALENDAR".to_string(),
            properties: HashMap::new(),
            children: vec![component],
        };
        let serialized = serialize_ical(&[wrapped]);
        prop_assert!(serialized.contains("BEGIN:VCALENDAR"));
        prop_assert!(serialized.contains("END:VCALENDAR"));
    }

    #[test]
    fn get_first_prop_returns_first(props in prop::collection::vec(arb_ical_property(), 1..5)) {
        let mut props_map = HashMap::new();
        let name = props[0].name.clone();
        props_map.insert(name.clone(), props.clone());

        let component = IcalComponent {
            name: "VEVENT".to_string(),
            properties: props_map,
            children: vec![],
        };

        let first = get_first_prop(&component, &name);
        prop_assert!(first.is_some());
        prop_assert_eq!(&first.unwrap().value, &props[0].value);
    }

    #[test]
    fn get_all_props_returns_all(
        name in "[A-Z]{2,10}",
        count in 1usize..5,
    ) {
        let props: Vec<IcalProperty> = (0..count)
            .map(|i| IcalProperty {
                name: name.clone(),
                params: HashMap::new(),
                value: format!("val{i}"),
            })
            .collect();

        let mut props_map = HashMap::new();
        props_map.insert(name.clone(), props.clone());

        let component = IcalComponent {
            name: "VEVENT".to_string(),
            properties: props_map,
            children: vec![],
        };

        let all = get_all_props(&component, &name);
        prop_assert_eq!(all.len(), count);
    }
}
