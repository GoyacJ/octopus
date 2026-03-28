ALTER TABLE approval_requests
    ADD COLUMN target_ref TEXT NOT NULL DEFAULT '';

UPDATE approval_requests
SET target_ref = 'run:' || run_id
WHERE target_ref = '';

ALTER TABLE inbox_items
    ADD COLUMN target_ref TEXT NOT NULL DEFAULT '';

UPDATE inbox_items
SET target_ref = COALESCE(
    (
        SELECT approval_requests.target_ref
        FROM approval_requests
        WHERE approval_requests.id = inbox_items.approval_request_id
    ),
    'run:' || run_id
)
WHERE target_ref = '';

ALTER TABLE notifications
    ADD COLUMN target_ref TEXT NOT NULL DEFAULT '';

UPDATE notifications
SET target_ref = COALESCE(
    (
        SELECT approval_requests.target_ref
        FROM approval_requests
        WHERE approval_requests.id = notifications.approval_request_id
    ),
    'run:' || run_id
)
WHERE target_ref = '';

CREATE INDEX IF NOT EXISTS idx_approval_requests_type_target_status
    ON approval_requests(approval_type, target_ref, status);
