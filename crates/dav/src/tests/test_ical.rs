use crate::ical::*;

#[test]
fn test_parse_vevent() {
    let input = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:test-123\r\n\
SUMMARY:Team Meeting\r\n\
DESCRIPTION:Weekly sync\r\n\
DTSTART:20260427T140000Z\r\n\
DTEND:20260427T150000Z\r\n\
LOCATION:Conference Room\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    let comps = parse_ical(input).unwrap();
    assert_eq!(comps.len(), 1);
    assert_eq!(comps[0].name, "VCALENDAR");

    let events: Vec<_> = comps[0].children.iter().filter(|c| c.name == "VEVENT").collect();
    assert_eq!(events.len(), 1);

    let event = &events[0];
    assert_eq!(get_first_prop(event, "UID").unwrap().value, "test-123");
    assert_eq!(get_first_prop(event, "SUMMARY").unwrap().value, "Team Meeting");
    assert_eq!(get_first_prop(event, "DESCRIPTION").unwrap().value, "Weekly sync");
    assert_eq!(get_first_prop(event, "DTSTART").unwrap().value, "20260427T140000Z");
    assert_eq!(get_first_prop(event, "DTEND").unwrap().value, "20260427T150000Z");
    assert_eq!(get_first_prop(event, "LOCATION").unwrap().value, "Conference Room");
}

#[test]
fn test_parse_vtodo() {
    let input = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VTODO\r\n\
UID:todo-1\r\n\
SUMMARY:Buy groceries\r\n\
DESCRIPTION:Milk, bread, eggs\r\n\
DUE:20260501T170000Z\r\n\
STATUS:NEEDS-ACTION\r\n\
END:VTODO\r\n\
END:VCALENDAR\r\n";

    let comps = parse_ical(input).unwrap();
    let todos: Vec<_> = comps[0].children.iter().filter(|c| c.name == "VTODO").collect();
    assert_eq!(todos.len(), 1);

    let todo = &todos[0];
    assert_eq!(get_first_prop(todo, "UID").unwrap().value, "todo-1");
    assert_eq!(get_first_prop(todo, "SUMMARY").unwrap().value, "Buy groceries");
    assert_eq!(get_first_prop(todo, "DUE").unwrap().value, "20260501T170000Z");
    assert_eq!(get_first_prop(todo, "STATUS").unwrap().value, "NEEDS-ACTION");
}

#[test]
fn test_parse_vtimezone() {
    let input = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VTIMEZONE\r\n\
TZID:America/New_York\r\n\
BEGIN:STANDARD\r\n\
DTSTART:19701101T020000\r\n\
RRULE:FREQ=YEARLY;BYDAY=1SU;BYMONTH=11\r\n\
TZOFFSETFROM:-0400\r\n\
TZOFFSETTO:-0500\r\n\
END:STANDARD\r\n\
BEGIN:DAYLIGHT\r\n\
DTSTART:19700308T020000\r\n\
RRULE:FREQ=YEARLY;BYDAY=2SU;BYMONTH=3\r\n\
TZOFFSETFROM:-0500\r\n\
TZOFFSETTO:-0400\r\n\
END:DAYLIGHT\r\n\
END:VTIMEZONE\r\n\
END:VCALENDAR\r\n";

    let comps = parse_ical(input).unwrap();
    let timezones: Vec<_> = comps[0].children.iter().filter(|c| c.name == "VTIMEZONE").collect();
    assert_eq!(timezones.len(), 1);

    let tz = &timezones[0];
    assert_eq!(get_first_prop(tz, "TZID").unwrap().value, "America/New_York");

    let standard: Vec<_> = tz.children.iter().filter(|c| c.name == "STANDARD").collect();
    assert_eq!(standard.len(), 1);
    assert_eq!(
        get_first_prop(&standard[0], "TZOFFSETFROM").unwrap().value,
        "-0400"
    );
    assert_eq!(
        get_first_prop(&standard[0], "TZOFFSETTO").unwrap().value,
        "-0500"
    );

    let daylight: Vec<_> = tz.children.iter().filter(|c| c.name == "DAYLIGHT").collect();
    assert_eq!(daylight.len(), 1);
    assert_eq!(
        get_first_prop(&daylight[0], "TZOFFSETTO").unwrap().value,
        "-0400"
    );
}

#[test]
fn test_parse_property_params() {
    let input = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;VALUE=DATE:20260427\r\n\
DTEND;VALUE=DATE:20260428\r\n\
SUMMARY;LANGUAGE=en:Birthday\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    let comps = parse_ical(input).unwrap();
    let event = &comps[0].children[0];

    let dtstart = get_first_prop(event, "DTSTART").unwrap();
    assert_eq!(dtstart.params.get("VALUE").unwrap(), "DATE");
    assert_eq!(dtstart.value, "20260427");

    let summary = get_first_prop(event, "SUMMARY").unwrap();
    assert_eq!(summary.params.get("LANGUAGE").unwrap(), "en");
    assert_eq!(summary.value, "Birthday");
}

#[test]
fn test_serialize_roundtrip() {
    let input = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:roundtrip-test\r\n\
SUMMARY:Test Event\r\n\
DTSTART:20260427T100000Z\r\n\
DTEND:20260427T110000Z\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    let comps = parse_ical(input).unwrap();
    let output = serialize_ical(&comps);

    let comps2 = parse_ical(&output).unwrap();
    assert_eq!(comps.len(), comps2.len());
    assert_eq!(comps[0].name, comps2[0].name);

    let event1 = &comps[0].children[0];
    let event2 = &comps2[0].children[0];
    assert_eq!(
        get_first_prop(event1, "UID").unwrap().value,
        get_first_prop(event2, "UID").unwrap().value
    );
    assert_eq!(
        get_first_prop(event1, "SUMMARY").unwrap().value,
        get_first_prop(event2, "SUMMARY").unwrap().value
    );
}

#[test]
fn test_line_unfolding() {
    let input = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nDESCRIPTION:This is a long \r\n description that is folded\r\nUID:fold-test\r\nSUMMARY:Folded Event\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    let comps = parse_ical(input).unwrap();
    let event = &comps[0].children[0];
    assert_eq!(
        get_first_prop(event, "DESCRIPTION").unwrap().value,
        "This is a long description that is folded"
    );
}

#[test]
fn test_parse_multiple_events() {
    let input = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:event-1\r\n\
SUMMARY:Event One\r\n\
DTSTART:20260427T090000Z\r\n\
DTEND:20260427T100000Z\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:event-2\r\n\
SUMMARY:Event Two\r\n\
DTSTART:20260427T140000Z\r\n\
DTEND:20260427T150000Z\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    let comps = parse_ical(input).unwrap();
    let events: Vec<_> = comps[0].children.iter().filter(|c| c.name == "VEVENT").collect();
    assert_eq!(events.len(), 2);
    assert_eq!(get_first_prop(&events[0], "UID").unwrap().value, "event-1");
    assert_eq!(get_first_prop(&events[1], "UID").unwrap().value, "event-2");
}

#[test]
fn test_parse_rrule() {
    let input = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
UID:rrule-test\r\n\
SUMMARY:Recurring Meeting\r\n\
DTSTART:20260427T100000Z\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20260630T235959Z\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    let comps = parse_ical(input).unwrap();
    let event = &comps[0].children[0];
    let rrule = get_first_prop(event, "RRULE").unwrap();
    assert_eq!(rrule.value, "FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20260630T235959Z");
}
