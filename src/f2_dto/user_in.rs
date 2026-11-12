//! Input DTOs for the `user` entity. Each DTO validates its own payload.


use serde::Deserialize;

use super::validate::Validate;
use crate::error::AppError;

const USERNAME_LEN: std::ops::RangeInclusive<usize> = 3..=32;
const PASSWORD_MIN_LEN: usize = 8;
const PASSWORD_MAX_LEN: usize = 128;
const EMAIL_MAX_LEN: usize = 254;

/// Returns the canonical form of a username (leading and trailing whitespace trimmed).
pub fn normalize_username(username: &str) -> &str {
    username.trim()
}

/// Returns the canonical form of an email (trimmed and lowercased).
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

// ─────────────────────────────── Register ───────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl Validate for RegisterRequest {
    fn validate(&self) -> Result<(), AppError> {
        check_username(&self.username)?;
        check_email(&self.email)?;
        check_password(&self.password)
    }
}

// ─────────────────────────────── Login ───────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl Validate for LoginRequest {
    fn validate(&self) -> Result<(), AppError> {
        if self.email.trim().is_empty() || self.password.is_empty() {
            return Err(invalid("email and password required"));
        }
        Ok(())
    }
}

// ─────────────────────────────── Update ───────────────────────────────

/// All fields are optional: only provided fields are updated.
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
}

impl Validate for UpdateUserRequest {
    fn validate(&self) -> Result<(), AppError> {
        if self.username.is_none() && self.email.is_none() && self.password.is_none() {
            return Err(invalid("no fields to update"));
        }
        if let Some(username) = &self.username {
            check_username(username)?;
        }
        if let Some(email) = &self.email {
            check_email(email)?;
        }
        if let Some(password) = &self.password {
            check_password(password)?;
        }
        Ok(())
    }
}

// ─────────────────────────────── Règles ───────────────────────────────

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.to_string())
}

fn check_username(raw: &str) -> Result<(), AppError> {
    let username = normalize_username(raw);
    if !USERNAME_LEN.contains(&username.chars().count()) {
        return Err(invalid("username: must be between 3 and 32 characters"));
    }
    if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(invalid("username: only letters, digits, '_' and '-' allowed"));
    }
    Ok(())
}

fn check_email(raw: &str) -> Result<(), AppError> {
    let email = normalize_email(raw);
    let err = || invalid("invalid email");

    if email.len() > EMAIL_MAX_LEN || email.chars().any(char::is_whitespace) {
        return Err(err());
    }
    let (local, domain) = email.split_once('@').ok_or_else(err)?;
    let domain_ok = domain.contains('.')
        && !domain.contains('@')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains("..");
    if local.is_empty() || !domain_ok {
        return Err(err());
    }
    Ok(())
}

fn check_password(password: &str) -> Result<(), AppError> {
    let len = password.chars().count();
    if len < PASSWORD_MIN_LEN {
        return Err(invalid("password: 8 characters minimum"));
    }
    if password.len() > PASSWORD_MAX_LEN {
        return Err(invalid("password: 128 bytes maximum"));
    }
    let has_letter = password.chars().any(char::is_alphabetic);
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !(has_letter && has_digit) {
        return Err(invalid("password: at least one letter and one digit required"));
    }
    Ok(())
}

// ═══════════════════════════════ UNIT TEST ═══════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    fn register(username: &str, email: &str, password: &str) -> RegisterRequest {
        RegisterRequest { username: username.into(), email: email.into(), password: password.into() }
    }

    #[test]
    fn valid_registration_passes() {
        assert!(register("scratch_42", "a@b.fr", "password1").validate().is_ok());
    }

    #[test]
    fn username_rules() {
        assert!(check_username("ab").is_err());
        assert!(check_username(&"a".repeat(33)).is_err());
        assert!(check_username("bad name").is_err());
        assert!(check_username("é-accent").is_err());
        assert!(check_username("ok-name_1").is_ok());
        assert!(check_username("  padded  ").is_ok(), "leading/trailing whitespace is trimmed");
    }

    #[test]
    fn email_rules() {
        for bad in ["", "plain", "@b.fr", "a@b", "a@@b.fr", "a b@c.fr", "a@.fr", "a@b..fr", "a@b.fr."] {
            assert!(check_email(bad).is_err(), "{bad:?} should be rejected");
        }
        assert!(check_email("first.last+tag@school42.fr").is_ok());
        assert!(check_email("  Uppercase@Example.FR ").is_ok());
    }

    #[test]
    fn password_rules() {
        assert!(check_password("short1").is_err());
        assert!(check_password("withoutanydigit").is_err());
        assert!(check_password("12345678").is_err());
        assert!(check_password(&format!("a1{}", "x".repeat(200))).is_err());
        assert!(check_password("password1").is_ok());
    }

    #[test]
    fn normalization() {
        assert_eq!(normalize_username("  bob "), "bob");
        assert_eq!(normalize_email("  Bob@Ex.FR "), "bob@ex.fr");
    }

    #[test]
    fn login_requires_both_fields() {
        let login = |email: &str, password: &str| LoginRequest { email: email.into(), password: password.into() };
        assert!(login("", "x").validate().is_err());
        assert!(login("a@b.fr", "").validate().is_err());
        assert!(login("a@b.fr", "x").validate().is_ok());
    }

    #[test]
    fn update_rules() {
        let update = |u: Option<&str>, e: Option<&str>, p: Option<&str>| UpdateUserRequest {
            username: u.map(Into::into),
            email: e.map(Into::into),
            password: p.map(Into::into),
        };
        assert!(update(None, None, None).validate().is_err(), "nothing to update");
        assert!(update(Some("new_name"), None, None).validate().is_ok());
        assert!(update(None, Some("nope"), None).validate().is_err());
        assert!(update(None, None, Some("weak")).validate().is_err());
    }

}
