# Incident Severity Levels

## Level 1: Critical

**Definition:** Complete service outage affecting all users, data loss, or security breach.

**Examples:**
- Database cluster failure
- API returning 500 errors for all requests
- Data breach detected
- Complete network partition

**Response:**
- Immediate page to on-call engineer
- All-hands response
- Customer notification within 15 minutes
- Executive update every 30 minutes

**Resolution Target:** 1 hour

## Level 2: High

**Definition:** Major feature degradation affecting significant portion of users.

**Examples:**
- Single node failure in cluster
- API latency > 5 seconds
- Authentication service down
- Memory leak causing OOM

**Response:**
- Page to on-call engineer
- Engineering team notified
- Customer notification within 1 hour
- Executive update every hour

**Resolution Target:** 4 hours

## Level 3: Medium

**Definition:** Minor feature degradation affecting some users.

**Examples:**
- Non-critical API errors
- Performance degradation
- Minor bug affecting subset of users
- Monitoring gaps

**Response:**
- Slack notification to on-call
- Engineering team notified
- Customer notification if widespread
- Update every 4 hours

**Resolution Target:** 24 hours

## Level 4: Low

**Definition:** Minor issues with workarounds available.

**Examples:**
- Cosmetic bugs
- Documentation errors
- Non-critical feature requests
- Minor performance issues

**Response:**
- Email notification
- Tracked in backlog
- Fixed in next sprint

**Resolution Target:** 1 week
