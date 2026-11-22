-- Run the complete script on one connection. All values are synthetic test data.
BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY;

-- A doubled quote and Unicode must survive statement splitting.
SELECT
  set_config('script_test.email', 'o''connor测试@example.com', true) AS email,
  set_config('script_test.unit', 'USD', true) AS unit,
  set_config('script_test.request_id', '', true) AS request_id,
  set_config('script_test.external_ref', '', true) AS external_ref;

SELECT current_database() AS database_name,
       current_setting('transaction_read_only') AS transaction_read_only,
       now() AS checked_at;

SELECT u.id AS customer_id, u.email, b.label, b.state, b.score,
       b.region, b.profile_type, b.display_name,
       b.reference_code, b.contact_email
FROM sample_customers u LEFT JOIN sample_profiles b ON b.customer_id = u.id
WHERE u.email = current_setting('script_test.email');

SELECT l.id, l.customer_id, l.state, l.unit, l.amount, l.source,
       l.external_ref, l.source_ref, l.match_key, l.created_at
FROM sample_events l JOIN sample_customers u ON u.id = l.customer_id
WHERE u.email = current_setting('script_test.email') AND l.type = 'credit'
  AND (current_setting('script_test.external_ref') = ''
       OR l.external_ref = current_setting('script_test.external_ref'))
ORDER BY l.created_at DESC LIMIT 100;

SELECT t.id, t.customer_id, t.category, t.state, t.amount, t.unit,
       t.payload, t.outcome, t.owner_id, t.created_at, t.completed_at
FROM sample_requests t JOIN sample_customers u ON u.id = t.customer_id
WHERE u.email = current_setting('script_test.email')
  AND (current_setting('script_test.request_id') = ''
       OR t.id::text = current_setting('script_test.request_id'))
ORDER BY t.created_at DESC LIMIT 100;

SELECT current_setting('script_test.unit') AS unit,
       COALESCE(SUM(l.amount), 0) AS total_amount
FROM sample_events l JOIN sample_customers u ON u.id = l.customer_id
WHERE u.email = current_setting('script_test.email')
  AND l.unit = current_setting('script_test.unit')
  AND l.state = 'success' AND l.type <> 'excluded';

SELECT l.id, l.customer_id, l.type, l.state, l.amount, l.unit, l.match_key
FROM sample_events l JOIN sample_customers u ON u.id = l.customer_id
WHERE u.email = current_setting('script_test.email')
  AND current_setting('script_test.request_id') <> ''
  AND l.match_key = 'request:' || current_setting('script_test.request_id');

SELECT current_setting('script_test.request_id') AS request_id,
       current_setting('script_test.unit') AS unit,
       COUNT(*) AS matched_rows,
       COALESCE(SUM(l.amount), 0) AS matched_amount
FROM sample_events l JOIN sample_customers u ON u.id = l.customer_id
WHERE u.email = current_setting('script_test.email')
  AND current_setting('script_test.request_id') <> ''
  AND l.match_key = 'request:' || current_setting('script_test.request_id')
  AND l.type = 'adjustment' AND l.state = 'success'
  AND l.unit = current_setting('script_test.unit');

COMMIT;
