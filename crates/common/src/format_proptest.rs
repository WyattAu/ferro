use proptest::prelude::*;

use crate::format::format_size;

proptest! {
    #[test]
    fn format_size_never_empty(bytes in 0u64..) {
        let result = format_size(bytes);
        prop_assert!(!result.is_empty());
    }

    #[test]
    fn format_size_always_contains_unit(bytes in 0u64..) {
        let result = format_size(bytes);
        prop_assert!(
            result.contains("B") || result.contains("KB") || result.contains("MB")
                || result.contains("GB") || result.contains("TB"),
            "format_size({}) = {:?} does not contain a unit",
            bytes, result
        );
    }

    #[test]
    fn format_size_zero_is_zero_b(_bytes in 0u64..1) {
        let result = format_size(0);
        prop_assert_eq!(result, "0 B");
    }

    #[test]
    fn format_size_bytes_range(bytes in 0u64..1024) {
        let result = format_size(bytes);
        prop_assert!(result.ends_with(" B"), "expected B suffix, got {:?}", result);
    }

    #[test]
    fn format_size_kb_range(bytes in 1024u64..1_048_576) {
        let result = format_size(bytes);
        prop_assert!(result.ends_with(" KB"), "expected KB suffix, got {:?}", result);
    }

    #[test]
    fn format_size_mb_range(bytes in 1_048_576u64..1_073_741_824) {
        let result = format_size(bytes);
        prop_assert!(result.ends_with(" MB"), "expected MB suffix, got {:?}", result);
    }

    #[test]
    fn format_size_gb_range(bytes in 1_073_741_824u64..1_099_511_627_776) {
        let result = format_size(bytes);
        prop_assert!(result.ends_with(" GB"), "expected GB suffix, got {:?}", result);
    }

    #[test]
    fn format_size_tb_range(bytes in 1_099_511_627_776u64..) {
        let result = format_size(bytes);
        prop_assert!(result.ends_with(" TB"), "expected TB suffix, got {:?}", result);
    }
}
