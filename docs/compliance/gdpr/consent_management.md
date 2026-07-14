# Consent Management

## Consent Types

### Explicit Consent
- **Use Case:** Marketing communications, analytics
- **Collection:** Opt-in checkbox with clear language
- **Withdrawal:** One-click unsubscribe

### Implicit Consent
- **Use Case:** Essential services, security logging
- **Collection:** Terms of service acceptance
- **Withdrawal:** Account deletion

## Consent Records

### Data Fields
- User ID
- Consent type
- Consent timestamp
- Withdrawal timestamp (if applicable)
- IP address
- User agent

### Storage
- Encrypted database storage
- Immutable audit trail
- Retention: Account lifetime + 12 months

## Consent Withdrawal

### Process
1. User requests withdrawal
2. System processes withdrawal
3. Data deletion initiated
4. Confirmation sent to user

### Impact
- Service continuity affected
- Data deletion within 30 days
- Legal obligations preserved
