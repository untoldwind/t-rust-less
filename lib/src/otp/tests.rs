use super::{OTPAlgorithm, OTPAuthUrl};
use spectral::prelude::*;

#[test]
fn test_totp_std() {
  let totp_url = "otpauth://totp/Example:someone@somewhere.com?secret=JBSWY3DPEHPK3PXP&issuer=Example";
  let otpauth = OTPAuthUrl::parse(totp_url).unwrap();

  assert_that(&otpauth.algorithm).is_equal_to(OTPAlgorithm::SHA1);
  assert_that(&otpauth.digits).is_equal_to(6);
  assert_that(&otpauth.issuer).is_equal_to(Some("Example".to_string()));
  assert_that(&otpauth.account_name).is_equal_to("someone@somewhere.com".to_string());

  assert_that(&otpauth.generate(1_556_733_311)).is_equal_to(("184557".to_string(), 1_556_733_330));
  assert_that(&otpauth.generate(1_556_733_406)).is_equal_to(("757120".to_string(), 1_556_733_420));

  assert_that(&otpauth.to_url())
    .is_equal_to("otpauth://totp/Example:someone%40somewhere.com?secret=JBSWY3DPEHPK3PXP&issuer=Example".to_string());
}

#[test]
fn test_totp_long() {
  let totp_url = "otpauth://totp/someone@somewhere.com?secret=LPD4D5FLWUBYFEB66SKYQGJBDS5HWYNT&period=60&digits=8";
  let otpauth = OTPAuthUrl::parse(totp_url).unwrap();

  assert_that(&otpauth.algorithm).is_equal_to(OTPAlgorithm::SHA1);
  assert_that(&otpauth.digits).is_equal_to(8);
  assert_that(&otpauth.issuer).is_none();
  assert_that(&otpauth.account_name).is_equal_to("someone@somewhere.com".to_string());

  assert_that(&otpauth.generate(1_556_733_830)).is_equal_to(("03744419".to_string(), 1_556_733_840));
  assert_that(&otpauth.generate(1_556_733_904)).is_equal_to(("84237990".to_string(), 1_556_733_960));

  assert_that(&otpauth.to_url()).is_equal_to(
    "otpauth://totp/someone%40somewhere.com?secret=LPD4D5FLWUBYFEB66SKYQGJBDS5HWYNT&period=60&digits=8".to_string(),
  );
}
