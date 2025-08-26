//! Alert management implementation for metrics-based alerting

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Alert manager for metrics-based alerting
pub struct AlertManager {
    alert_rules: Vec<AlertRule>,
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    alert_history: Arc<RwLock<Vec<Alert>>>,
    notification_channels: Vec<NotificationChannel>,
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule ID
    pub id: String,

    /// Rule name
    pub name: String,

    /// Rule description
    pub description: String,

    /// Metric to monitor
    pub metric: String,

    /// Alert condition
    pub condition: AlertCondition,

    /// Alert severity
    pub severity: AlertSeverity,

    /// Cooldown period in seconds
    pub cooldown_seconds: u64,

    /// Enabled flag
    pub enabled: bool,
}

/// Alert condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// Greater than threshold
    GreaterThan(f64),

    /// Less than threshold
    LessThan(f64),

    /// Equals threshold
    Equals(f64),

    /// Not equals threshold
    NotEquals(f64),

    /// Between range (inclusive)
    Between(f64, f64),

    /// Outside range
    Outside(f64, f64),
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

impl fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
            AlertSeverity::Emergency => write!(f, "EMERGENCY"),
        }
    }
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertStatus {
    Active,
    Resolved,
    Acknowledged,
}

/// Alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert ID
    pub id: String,

    /// Rule ID that triggered this alert
    pub rule_id: String,

    /// Alert name
    pub name: String,

    /// Alert description
    pub description: String,

    /// Alert severity
    pub severity: AlertSeverity,

    /// Alert status
    pub status: AlertStatus,

    /// Current metric value
    pub current_value: f64,

    /// Threshold value
    pub threshold_value: f64,

    /// Alert timestamp
    pub timestamp: u64,

    /// Resolved timestamp
    pub resolved_timestamp: Option<u64>,

    /// Acknowledged timestamp
    pub acknowledged_timestamp: Option<u64>,

    /// Acknowledged by
    pub acknowledged_by: Option<String>,

    /// Additional context
    pub context: HashMap<String, String>,
}

/// Notification channel types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    /// Log-based notifications
    Log,

    /// HTTP webhook notifications
    Webhook {
        url: String,
        headers: HashMap<String, String>,
    },

    /// Email notifications
    Email {
        smtp_server: String,
        smtp_port: u16,
        username: String,
        password: String,
        from_address: String,
        to_addresses: Vec<String>,
    },

    /// Slack notifications
    Slack {
        webhook_url: String,
        channel: String,
    },

    /// Custom notification channel
    Custom {
        name: String,
        config: HashMap<String, String>,
    },
}

impl AlertManager {
    /// Create new alert manager
    pub fn new() -> Self {
        let mut alert_manager = Self {
            alert_rules: Vec::new(),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            notification_channels: Vec::new(),
        };

        // Add default notification channel (log)
        alert_manager.notification_channels.push(NotificationChannel::Log);

        alert_manager
    }

    /// Add alert rule
    pub fn add_alert_rule(&mut self, rule: AlertRule) {
        let rule_name = rule.name.clone();
        self.alert_rules.push(rule);
        info!("Added alert rule: {}", rule_name);
    }

    /// Remove alert rule
    pub fn remove_alert_rule(&mut self, rule_id: &str) {
        self.alert_rules.retain(|rule| rule.id != rule_id);
        info!("Removed alert rule: {}", rule_id);
    }

    /// Get alert rules
    pub fn get_alert_rules(&self) -> &[AlertRule] {
        &self.alert_rules
    }

    /// Add notification channel
    pub fn add_notification_channel(&mut self, channel: NotificationChannel) {
        self.notification_channels.push(channel);
        info!("Added notification channel");
    }

    /// Check metrics against alert rules
    pub async fn check_alerts(&self, metrics: &super::types::AgentMetrics) -> Result<(), Box<dyn std::error::Error>> {
        for rule in &self.alert_rules {
            if !rule.enabled {
                continue;
            }

            let metric_value = self.get_metric_value(&rule.metric, metrics);
            let should_alert = self.evaluate_condition(&rule.condition, metric_value);

            if should_alert {
                self.trigger_alert(rule, metric_value).await?;
            } else {
                self.resolve_alert(rule).await?;
            }
        }

        Ok(())
    }

    /// Get metric value from metrics
    fn get_metric_value(&self, metric_name: &str, metrics: &super::types::AgentMetrics) -> f64 {
        match metric_name {
            "cpu_percent" => metrics.resources.cpu_percent,
            "memory_percent" => metrics.resources.memory_percent,
            "disk_percent" => metrics.resources.disk_percent,
            "error_rate" => metrics.errors.error_rate,
            "requests_per_second" => metrics.performance.requests_per_second,
            "avg_response_time_ms" => metrics.performance.avg_response_time_ms,
            "tasks_pending" => metrics.tasks.pending as f64,
            "tasks_active" => metrics.tasks.active as f64,
            _ => 0.0, // Unknown metric
        }
    }

    /// Evaluate alert condition
    fn evaluate_condition(&self, condition: &AlertCondition, value: f64) -> bool {
        match condition {
            AlertCondition::GreaterThan(threshold) => value > *threshold,
            AlertCondition::LessThan(threshold) => value < *threshold,
            AlertCondition::Equals(threshold) => (value - threshold).abs() < f64::EPSILON,
            AlertCondition::NotEquals(threshold) => (value - threshold).abs() >= f64::EPSILON,
            AlertCondition::Between(min, max) => value >= *min && value <= *max,
            AlertCondition::Outside(min, max) => value < *min || value > *max,
        }
    }

    /// Trigger an alert
    async fn trigger_alert(&self, rule: &AlertRule, current_value: f64) -> Result<(), Box<dyn std::error::Error>> {
        let alert_id = format!("{}-{}", rule.id, super::types::current_timestamp());
        
        let alert = Alert {
            id: alert_id.clone(),
            rule_id: rule.id.clone(),
            name: rule.name.clone(),
            description: rule.description.clone(),
            severity: rule.severity.clone(),
            status: AlertStatus::Active,
            current_value,
            threshold_value: self.get_threshold_value(&rule.condition),
            timestamp: super::types::current_timestamp(),
            resolved_timestamp: None,
            acknowledged_timestamp: None,
            acknowledged_by: None,
            context: HashMap::new(),
        };

        // Add to active alerts
        {
            let mut active_alerts = self.active_alerts.write().await;
            active_alerts.insert(alert_id.clone(), alert.clone());
        }

        // Send notifications
        self.send_notifications(&alert).await?;

        info!("Alert triggered: {} ({} = {})", rule.name, rule.metric, current_value);
        Ok(())
    }

    /// Resolve an alert
    async fn resolve_alert(&self, rule: &AlertRule) -> Result<(), Box<dyn std::error::Error>> {
        let mut active_alerts = self.active_alerts.write().await;
        
        if let Some(alert) = active_alerts.remove(&rule.id) {
            let mut resolved_alert = alert.clone();
            resolved_alert.status = AlertStatus::Resolved;
            resolved_alert.resolved_timestamp = Some(super::types::current_timestamp());

            // Add to history
            {
                let mut alert_history = self.alert_history.write().await;
                alert_history.push(resolved_alert);
            }

            info!("Alert resolved: {}", rule.name);
        }

        Ok(())
    }

    /// Get threshold value from condition
    fn get_threshold_value(&self, condition: &AlertCondition) -> f64 {
        match condition {
            AlertCondition::GreaterThan(threshold) => *threshold,
            AlertCondition::LessThan(threshold) => *threshold,
            AlertCondition::Equals(threshold) => *threshold,
            AlertCondition::NotEquals(threshold) => *threshold,
            AlertCondition::Between(min, _) => *min,
            AlertCondition::Outside(min, _) => *min,
        }
    }

    /// Send notifications for an alert
    async fn send_notifications(&self, alert: &Alert) -> Result<(), Box<dyn std::error::Error>> {
        for channel in &self.notification_channels {
            match channel {
                NotificationChannel::Log => {
                    self.send_log_notification(alert).await?;
                }
                NotificationChannel::Webhook { url, headers } => {
                    self.send_webhook_notification(alert, url, headers).await?;
                }
                NotificationChannel::Slack { webhook_url, channel } => {
                    self.send_slack_notification(alert, webhook_url, channel).await?;
                }
                _ => {
                    warn!("Unsupported notification channel type");
                }
            }
        }

        Ok(())
    }

    /// Send log notification
    async fn send_log_notification(&self, alert: &Alert) -> Result<(), Box<dyn std::error::Error>> {
        match alert.severity {
            AlertSeverity::Info => {
                info!("ALERT [{}]: {} - {}", alert.severity, alert.name, alert.description);
            }
            AlertSeverity::Warning => {
                warn!("ALERT [{}]: {} - {}", alert.severity, alert.name, alert.description);
            }
            AlertSeverity::Critical | AlertSeverity::Emergency => {
                error!("ALERT [{}]: {} - {}", alert.severity, alert.name, alert.description);
            }
        }
        Ok(())
    }

    /// Send webhook notification
    async fn send_webhook_notification(
        &self,
        alert: &Alert,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation would use reqwest or similar HTTP client
        info!("Sending webhook notification to {} for alert: {}", url, alert.name);
        Ok(())
    }

    /// Send Slack notification
    async fn send_slack_notification(
        &self,
        alert: &Alert,
        webhook_url: &str,
        channel: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation would use reqwest to send Slack webhook
        info!("Sending Slack notification to #{} for alert: {}", channel, alert.name);
        Ok(())
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.values().cloned().collect()
    }

    /// Get alert history
    pub async fn get_alert_history(&self) -> Vec<Alert> {
        let alert_history = self.alert_history.read().await;
        alert_history.clone()
    }

    /// Acknowledge alert
    pub async fn acknowledge_alert(&mut self, alert_id: &str, acknowledged_by: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut active_alerts = self.active_alerts.write().await;
        
        if let Some(alert) = active_alerts.get_mut(alert_id) {
            alert.status = AlertStatus::Acknowledged;
            alert.acknowledged_timestamp = Some(super::types::current_timestamp());
            alert.acknowledged_by = Some(acknowledged_by);
            
            info!("Alert acknowledged: {}", alert_id);
        }

        Ok(())
    }
}
