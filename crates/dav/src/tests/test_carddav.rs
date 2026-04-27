use crate::store::*;

fn sample_vcard(uid: &str, fn_name: &str, family: &str, given: &str) -> String {
    format!(
        "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
UID:{}\r\n\
FN:{}\r\n\
N:{};{};;;\r\n\
END:VCARD\r\n",
        uid, fn_name, family, given
    )
}

#[tokio::test]
async fn test_create_and_list_address_books() {
    let store = InMemoryAddressBookStore::new();
    let book = store
        .create_address_book("user1", "Contacts")
        .await
        .unwrap();
    assert_eq!(book.name, "Contacts");
    assert_eq!(book.principal, "user1");

    let books = store.list_address_books("user1").await;
    assert_eq!(books.len(), 1);
    assert_eq!(books[0].id, book.id);
}

#[tokio::test]
async fn test_delete_address_book() {
    let store = InMemoryAddressBookStore::new();
    let book = store
        .create_address_book("user1", "To Delete")
        .await
        .unwrap();

    store.delete_address_book("user1", &book.id).await.unwrap();
    let books = store.list_address_books("user1").await;
    assert!(books.is_empty());
}

#[tokio::test]
async fn test_get_address_book() {
    let store = InMemoryAddressBookStore::new();
    let book = store
        .create_address_book("user1", "My Book")
        .await
        .unwrap();

    let fetched = store.get_address_book("user1", &book.id).await;
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "My Book");

    let not_found = store.get_address_book("user1", "nonexistent").await;
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_create_and_list_contacts() {
    let store = InMemoryAddressBookStore::new();
    let book = store
        .create_address_book("user1", "Contacts")
        .await
        .unwrap();

    let vcard = sample_vcard("c-1", "Alice Smith", "Smith", "Alice");
    store.create_contact(&book.id, &vcard).await.unwrap();

    let contacts = store.list_contacts(&book.id).await;
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].uid, "c-1");
}

#[tokio::test]
async fn test_get_contact() {
    let store = InMemoryAddressBookStore::new();
    let book = store
        .create_address_book("user1", "Contacts")
        .await
        .unwrap();

    let vcard = sample_vcard("c-2", "Bob Jones", "Jones", "Bob");
    store.create_contact(&book.id, &vcard).await.unwrap();

    let contact = store.get_contact(&book.id, "c-2").await;
    assert!(contact.is_some());
    assert_eq!(contact.unwrap().uid, "c-2");
}

#[tokio::test]
async fn test_update_contact() {
    let store = InMemoryAddressBookStore::new();
    let book = store
        .create_address_book("user1", "Contacts")
        .await
        .unwrap();

    let vcard = sample_vcard("c-3", "Original Name", "Orig", "Name");
    store.create_contact(&book.id, &vcard).await.unwrap();

    let updated_vcard = sample_vcard("c-3", "Updated Name", "Orig", "Name");
    let contact = store
        .update_contact(&book.id, "c-3", &updated_vcard)
        .await
        .unwrap();
    assert!(contact.vcard_data.contains("Updated Name"));
}

#[tokio::test]
async fn test_delete_contact() {
    let store = InMemoryAddressBookStore::new();
    let book = store
        .create_address_book("user1", "Contacts")
        .await
        .unwrap();

    let vcard = sample_vcard("c-4", "Delete Me", "Del", "Me");
    store.create_contact(&book.id, &vcard).await.unwrap();

    store.delete_contact(&book.id, "c-4").await.unwrap();
    let contacts = store.list_contacts(&book.id).await;
    assert!(contacts.is_empty());
}

#[tokio::test]
async fn test_contact_isolation() {
    let store = InMemoryAddressBookStore::new();
    let book1 = store
        .create_address_book("user1", "Book 1")
        .await
        .unwrap();
    let book2 = store
        .create_address_book("user1", "Book 2")
        .await
        .unwrap();

    let vcard = sample_vcard("iso-1", "Isolated", "Iso", "Lated");
    store.create_contact(&book1.id, &vcard).await.unwrap();

    let contacts1 = store.list_contacts(&book1.id).await;
    let contacts2 = store.list_contacts(&book2.id).await;
    assert_eq!(contacts1.len(), 1);
    assert_eq!(contacts2.len(), 0);
}

#[tokio::test]
async fn test_create_duplicate_contact_fails() {
    let store = InMemoryAddressBookStore::new();
    let book = store
        .create_address_book("user1", "Contacts")
        .await
        .unwrap();

    let vcard = sample_vcard("dup-1", "Duplicate", "Dup", "Licate");
    store.create_contact(&book.id, &vcard).await.unwrap();

    let result = store.create_contact(&book.id, &vcard).await;
    assert!(result.is_err());
}
