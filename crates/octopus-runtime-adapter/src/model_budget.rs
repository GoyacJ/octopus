use super::*;
use octopus_core::{BudgetAccountingMode, BudgetReservationStrategy, ConfiguredModelBudgetPolicy};

pub(super) const BUDGET_TRAFFIC_CLASS_INTERACTIVE_TURN: &str = "interactive_turn";
pub(super) const BUDGET_TRAFFIC_CLASS_PROBE: &str = "probe";

#[derive(Debug, Clone, Copy, Default)]
struct ConfiguredModelBudgetProjection {
    settled_tokens: u64,
    active_reserved_tokens: u64,
}

#[derive(Debug, Clone)]
struct ConfiguredModelBudgetReservation {
    configured_model_id: String,
    traffic_class: String,
    reserved_tokens: u64,
    status: String,
}

impl RuntimeAdapter {
    pub(super) fn load_configured_model_usage_map(&self) -> Result<HashMap<String, u64>, AppError> {
        let connection = self.open_db()?;
        let mut statement = connection
            .prepare(
                "SELECT configured_model_id, settled_tokens
                 FROM configured_model_budget_projections",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                let configured_model_id: String = row.get(0)?;
                let settled_tokens: i64 = row.get(1)?;
                Ok((configured_model_id, settled_tokens))
            })
            .map_err(|error| AppError::database(error.to_string()))?;

        let mut usage = HashMap::new();
        for row in rows {
            let (configured_model_id, settled_tokens) =
                row.map_err(|error| AppError::database(error.to_string()))?;
            usage.insert(configured_model_id, settled_tokens.max(0) as u64);
        }
        Ok(usage)
    }

    pub(super) fn reserve_configured_model_budget(
        &self,
        reservation_id: &str,
        configured_model: &ConfiguredModelRecord,
        traffic_class: &str,
        reserved_at: u64,
    ) -> Result<(), AppError> {
        let Some(budget_policy) = configured_model.budget_policy.as_ref() else {
            return Ok(());
        };
        let Some(total_budget_tokens) = budget_policy.total_budget_tokens else {
            return Ok(());
        };
        if let Some(message) = unsupported_budget_accounting_mode_message(
            &configured_model.configured_model_id,
            budget_policy,
        ) {
            return Err(AppError::invalid_input(message));
        }
        if !configured_model_budget_applies_to_traffic_class(configured_model, traffic_class) {
            return Ok(());
        }

        let mut connection = self.open_db()?;
        let transaction = connection
            .transaction()
            .map_err(|error| AppError::database(error.to_string()))?;
        let projection =
            load_budget_projection(&transaction, &configured_model.configured_model_id)?
                .unwrap_or_default();
        let existing = load_budget_reservation(&transaction, reservation_id)?;
        if existing
            .as_ref()
            .is_some_and(|reservation| reservation.status == "active")
        {
            return Err(AppError::conflict(format!(
                "configured model budget reservation `{reservation_id}` is already active"
            )));
        }

        ensure_configured_model_budget_available(
            &configured_model.configured_model_id,
            projection,
            total_budget_tokens,
        )?;

        let reserved_tokens = match configured_model
            .budget_policy
            .as_ref()
            .map(|policy| policy.reservation_strategy)
            .unwrap_or(BudgetReservationStrategy::None)
        {
            BudgetReservationStrategy::None => 0,
            BudgetReservationStrategy::Fixed => {
                let unavailable_tokens = projection
                    .settled_tokens
                    .saturating_add(projection.active_reserved_tokens);
                total_budget_tokens.saturating_sub(unavailable_tokens)
            }
        };

        if reserved_tokens == 0
            && configured_model
                .budget_policy
                .as_ref()
                .is_some_and(|policy| {
                    policy.reservation_strategy == BudgetReservationStrategy::Fixed
                })
        {
            return Err(AppError::invalid_input(format!(
                "configured model `{}` has reached its total token limit",
                configured_model.configured_model_id
            )));
        }

        let updated_projection = ConfiguredModelBudgetProjection {
            settled_tokens: projection.settled_tokens,
            active_reserved_tokens: projection
                .active_reserved_tokens
                .saturating_add(reserved_tokens),
        };
        upsert_budget_projection(
            &transaction,
            &configured_model.configured_model_id,
            updated_projection,
            reserved_at,
        )?;
        upsert_budget_reservation(
            &transaction,
            reservation_id,
            &configured_model.configured_model_id,
            traffic_class,
            reserved_tokens,
            "active",
            reserved_at,
            None,
            None,
        )?;
        transaction
            .commit()
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) fn settle_configured_model_budget_reservation(
        &self,
        reservation_id: &str,
        configured_model_id: &str,
        settled_tokens: u32,
        settled_at: u64,
    ) -> Result<u64, AppError> {
        let mut connection = self.open_db()?;
        let transaction = connection
            .transaction()
            .map_err(|error| AppError::database(error.to_string()))?;
        let Some(reservation) = load_budget_reservation(&transaction, reservation_id)? else {
            let projection =
                load_budget_projection(&transaction, configured_model_id)?.unwrap_or_default();
            return Ok(projection.settled_tokens);
        };
        let reserved_tokens = reservation.reserved_tokens;
        let projection =
            load_budget_projection(&transaction, configured_model_id)?.unwrap_or_default();
        let updated_projection = ConfiguredModelBudgetProjection {
            settled_tokens: projection
                .settled_tokens
                .saturating_add(u64::from(settled_tokens)),
            active_reserved_tokens: projection
                .active_reserved_tokens
                .saturating_sub(reserved_tokens),
        };
        upsert_budget_projection(
            &transaction,
            configured_model_id,
            updated_projection,
            settled_at,
        )?;
        upsert_budget_reservation(
            &transaction,
            reservation_id,
            &reservation.configured_model_id,
            &reservation.traffic_class,
            reservation.reserved_tokens,
            "settled",
            settled_at,
            None,
            Some(settled_at),
        )?;
        upsert_budget_settlement(
            &transaction,
            reservation_id,
            configured_model_id,
            &reservation.traffic_class,
            u64::from(settled_tokens),
            settled_at,
        )?;
        transaction
            .commit()
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(updated_projection.settled_tokens)
    }

    pub(super) fn release_configured_model_budget_reservation(
        &self,
        reservation_id: &str,
        released_at: u64,
    ) -> Result<(), AppError> {
        let mut connection = self.open_db()?;
        let transaction = connection
            .transaction()
            .map_err(|error| AppError::database(error.to_string()))?;
        let Some(reservation) = load_budget_reservation(&transaction, reservation_id)? else {
            return Ok(());
        };
        if reservation.status != "active" {
            return Ok(());
        }

        let projection = load_budget_projection(&transaction, &reservation.configured_model_id)?
            .unwrap_or_default();
        let updated_projection = ConfiguredModelBudgetProjection {
            settled_tokens: projection.settled_tokens,
            active_reserved_tokens: projection
                .active_reserved_tokens
                .saturating_sub(reservation.reserved_tokens),
        };
        upsert_budget_projection(
            &transaction,
            &reservation.configured_model_id,
            updated_projection,
            released_at,
        )?;
        upsert_budget_reservation(
            &transaction,
            reservation_id,
            &reservation.configured_model_id,
            &reservation.traffic_class,
            reservation.reserved_tokens,
            "released",
            released_at,
            Some(released_at),
            None,
        )?;
        transaction
            .commit()
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) fn resolve_consumed_tokens(
        &self,
        configured_model: &ConfiguredModelRecord,
        response: &ModelExecutionResult,
    ) -> Result<Option<u32>, AppError> {
        match response.total_tokens {
            Some(total_tokens) => Ok(Some(total_tokens)),
            None if configured_model
                .budget_policy
                .as_ref()
                .and_then(|policy| policy.total_budget_tokens)
                .is_some() =>
            {
                Err(AppError::runtime(format!(
                    "configured model `{}` requires provider token usage for budget enforcement",
                    configured_model.configured_model_id
                )))
            }
            None => Ok(None),
        }
    }
}

pub(super) fn unsupported_budget_accounting_mode_message(
    configured_model_id: &str,
    budget_policy: &ConfiguredModelBudgetPolicy,
) -> Option<String> {
    if budget_policy.total_budget_tokens.is_some()
        && budget_policy.accounting_mode != BudgetAccountingMode::ProviderReported
    {
        return Some(format!(
            "configured model `{configured_model_id}` budgetPolicy.accountingMode `{}` is not supported for budget enforcement; use `provider_reported`",
            budget_accounting_mode_key(budget_policy.accounting_mode)
        ));
    }

    None
}

fn configured_model_budget_applies_to_traffic_class(
    configured_model: &ConfiguredModelRecord,
    traffic_class: &str,
) -> bool {
    configured_model
        .budget_policy
        .as_ref()
        .map(|policy| {
            policy.traffic_classes.is_empty()
                || policy
                    .traffic_classes
                    .iter()
                    .any(|entry| entry == traffic_class)
        })
        .unwrap_or(false)
}

fn ensure_configured_model_budget_available(
    configured_model_id: &str,
    projection: ConfiguredModelBudgetProjection,
    total_tokens: u64,
) -> Result<(), AppError> {
    let unavailable_tokens = projection
        .settled_tokens
        .saturating_add(projection.active_reserved_tokens);
    if unavailable_tokens >= total_tokens {
        return Err(AppError::invalid_input(format!(
            "configured model `{configured_model_id}` has reached its total token limit"
        )));
    }

    Ok(())
}

fn budget_accounting_mode_key(mode: BudgetAccountingMode) -> &'static str {
    match mode {
        BudgetAccountingMode::ProviderReported => "provider_reported",
        BudgetAccountingMode::Estimated => "estimated",
        BudgetAccountingMode::NonBillable => "non_billable",
    }
}

fn load_budget_projection(
    connection: &Connection,
    configured_model_id: &str,
) -> Result<Option<ConfiguredModelBudgetProjection>, AppError> {
    connection
        .query_row(
            "SELECT settled_tokens, active_reserved_tokens
             FROM configured_model_budget_projections
             WHERE configured_model_id = ?1",
            [configured_model_id],
            |row| {
                let settled_tokens: i64 = row.get(0)?;
                let active_reserved_tokens: i64 = row.get(1)?;
                Ok(ConfiguredModelBudgetProjection {
                    settled_tokens: settled_tokens.max(0) as u64,
                    active_reserved_tokens: active_reserved_tokens.max(0) as u64,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))
}

fn upsert_budget_projection(
    connection: &Connection,
    configured_model_id: &str,
    projection: ConfiguredModelBudgetProjection,
    updated_at: u64,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT INTO configured_model_budget_projections (
                configured_model_id,
                settled_tokens,
                active_reserved_tokens,
                updated_at
             ) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(configured_model_id)
             DO UPDATE SET
               settled_tokens = excluded.settled_tokens,
               active_reserved_tokens = excluded.active_reserved_tokens,
               updated_at = excluded.updated_at",
            params![
                configured_model_id,
                projection.settled_tokens as i64,
                projection.active_reserved_tokens as i64,
                updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

fn load_budget_reservation(
    connection: &Connection,
    reservation_id: &str,
) -> Result<Option<ConfiguredModelBudgetReservation>, AppError> {
    connection
        .query_row(
            "SELECT configured_model_id, traffic_class, reserved_tokens, status
             FROM configured_model_budget_reservations
             WHERE id = ?1",
            [reservation_id],
            |row| {
                let configured_model_id: String = row.get(0)?;
                let traffic_class: String = row.get(1)?;
                let reserved_tokens: i64 = row.get(2)?;
                let status: String = row.get(3)?;
                Ok(ConfiguredModelBudgetReservation {
                    configured_model_id,
                    traffic_class,
                    reserved_tokens: reserved_tokens.max(0) as u64,
                    status,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))
}

fn upsert_budget_reservation(
    connection: &Connection,
    reservation_id: &str,
    configured_model_id: &str,
    traffic_class: &str,
    reserved_tokens: u64,
    status: &str,
    updated_at: u64,
    released_at: Option<u64>,
    settled_at: Option<u64>,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT INTO configured_model_budget_reservations (
                id,
                configured_model_id,
                traffic_class,
                reserved_tokens,
                status,
                created_at,
                updated_at,
                released_at,
                settled_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8)
             ON CONFLICT(id)
             DO UPDATE SET
               configured_model_id = excluded.configured_model_id,
               traffic_class = excluded.traffic_class,
               reserved_tokens = excluded.reserved_tokens,
               status = excluded.status,
               updated_at = excluded.updated_at,
               released_at = excluded.released_at,
               settled_at = excluded.settled_at",
            params![
                reservation_id,
                configured_model_id,
                traffic_class,
                reserved_tokens as i64,
                status,
                updated_at as i64,
                released_at.map(|value| value as i64),
                settled_at.map(|value| value as i64),
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

fn upsert_budget_settlement(
    connection: &Connection,
    reservation_id: &str,
    configured_model_id: &str,
    traffic_class: &str,
    settled_tokens: u64,
    updated_at: u64,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT INTO configured_model_budget_settlements (
                reservation_id,
                configured_model_id,
                traffic_class,
                settled_tokens,
                created_at,
                updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(reservation_id)
             DO UPDATE SET
               configured_model_id = excluded.configured_model_id,
               traffic_class = excluded.traffic_class,
               settled_tokens = excluded.settled_tokens,
               updated_at = excluded.updated_at",
            params![
                reservation_id,
                configured_model_id,
                traffic_class,
                settled_tokens as i64,
                updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}
