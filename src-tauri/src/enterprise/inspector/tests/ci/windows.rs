mod setup1 {
    use super::super::super::super::{
        anti_virus_status, disk_encryption_status, os_name, os_version, part_of_domain,
        security_update_age_days,
    };

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_os_name() {
        assert_eq!(os_name().unwrap(), "Windows");
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_os_version() {
        assert_eq!(os_version().unwrap(), "11");
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_anti_virus_status_on() {
        assert_eq!(anti_virus_status().unwrap(), true);
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_part_of_domain_false() {
        assert_eq!(part_of_domain().unwrap(), false);
    }

    // #[test]
    // #[ignore = "CI posture testing only"]
    // fn test_security_update_age_days() {
    //     assert_eq!(security_update_age_days().unwrap(), 60);
    // }


    #[test]
    #[ignore = "CI posture testing only"]
    fn test_disk_encryption_status_unencrypted() {
        assert_eq!(disk_encryption_status().unwrap(), false);
    }
}

mod setup2 {
    use super::super::super::super::{
        anti_virus_status, disk_encryption_status, os_name, os_version, part_of_domain,
        security_update_age_days,
    };

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_os_name() {
        assert_eq!(os_name().unwrap(), "Windows");
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_os_version() {
        assert_eq!(os_version().unwrap(), "11");
    }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_anti_virus_status_off() {
        assert_eq!(anti_virus_status().unwrap(), false);
    }

    // #[test]
    // #[ignore = "CI posture testing only"]
    // fn test_part_of_domain_true() {
    //     assert_eq!(part_of_domain().unwrap(), true);
    // }

    // #[test]
    // #[ignore = "CI posture testing only"]
    // fn test_security_update_age_days() {
    //     assert_eq!(security_update_age_days().unwrap(), 60);
    // }

    #[test]
    #[ignore = "CI posture testing only"]
    fn test_disk_encryption_status_encrypted() {
        assert_eq!(disk_encryption_status().unwrap(), true);
    }

}
