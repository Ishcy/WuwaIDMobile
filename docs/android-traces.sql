-- Analisis read-only untuk satu trace/satu instance WuwaIDMobile + UserService.
-- Jalankan dengan Perfetto v58.2:
-- trace_processor query -f docs/android-traces.sql android_traces/pasang-terjemahan.pftrace
-- Tiap SELECT menghasilkan tabel CSV terpisah. Tidak menulis trace atau resource game.

CREATE PERFETTO TABLE wuwa_trace_processes AS
SELECT upid, pid, name, parent_upid, start_ts, end_ts
FROM process
WHERE cmdline IN ('com.titotfp.wuwaid', 'com.titotfp.wuwaid:wuwaid-storage');

CREATE PERFETTO TABLE wuwa_trace_calls AS
SELECT s.id, s.ts, s.dur, t.tid, t.name AS thread_name
FROM slice s
JOIN thread_track tt ON tt.id = s.track_id
JOIN thread t USING (utid)
WHERE t.upid = (
  SELECT MAX(upid) FROM wuwa_trace_processes WHERE name = 'com.titotfp.wuwaid'
)
AND s.name = 'binder transaction' AND s.dur > 0
AND EXTRACT_ARG(s.arg_set_id, 'destination process') = (
  SELECT pid FROM wuwa_trace_processes
  WHERE upid = (
    SELECT MAX(upid) FROM wuwa_trace_processes
    WHERE name = 'com.titotfp.wuwaid:wuwaid-storage'
  )
);

CREATE PERFETTO TABLE wuwa_trace_shells AS
SELECT upid, pid, start_ts, cmdline,
  CASE
    WHEN INSTR(cmdline, 'resource root is not readable') > 0 THEN 'root_readable'
    WHEN INSTR(cmdline, 'resource root is not writable') > 0 THEN 'root_writable'
    WHEN INSTR(cmdline, 'for entry in ') > 0 THEN 'list'
    WHEN INSTR(cmdline, 'text resource exceeds') > 0 THEN 'read_text'
    WHEN INSTR(cmdline, 'printf ''D') > 0 THEN 'stat'
    WHEN INSTR(cmdline, 'sha256sum') > 0 THEN 'hash'
    WHEN INSTR(cmdline, 'resource copy failed') > 0 THEN 'copy'
    WHEN INSTR(cmdline, 'resource rename failed') > 0 THEN 'move'
    WHEN INSTR(cmdline, 'resource removal failed') > 0 THEN 'remove'
    WHEN INSTR(cmdline, 'resource write failed') > 0 THEN 'write'
    WHEN INSTR(cmdline, 'unable to create owned resource directory') > 0 THEN 'mkdir'
    WHEN INSTR(cmdline, 'am force-stop ') > 0 THEN 'force_stop'
    ELSE 'unknown'
  END AS operation,
  SUBSTR(cmdline, INSTR(cmdline, ' wuwaid-storage ') + LENGTH(' wuwaid-storage ')) AS argv_tail
FROM process
WHERE parent_upid = (
  SELECT MAX(upid) FROM wuwa_trace_processes
  WHERE name = 'com.titotfp.wuwaid:wuwaid-storage'
);

CREATE PERFETTO TABLE wuwa_trace_operations AS
SELECT c.*, sh.pid AS shell_pid, sh.operation,
  CASE WHEN INSTR(sh.argv_tail, ' ') = 0 THEN ''
    ELSE SUBSTR(sh.argv_tail, INSTR(sh.argv_tail, ' ') + 1)
  END AS path_and_algorithm
FROM wuwa_trace_calls c
JOIN wuwa_trace_shells sh ON sh.start_ts >= c.ts AND sh.start_ts < c.ts + c.dur;

-- 1. Identitas proses, perangkat, dan integritas ingest.
SELECT * FROM wuwa_trace_processes ORDER BY upid;
SELECT name, str_value, int_value FROM metadata
WHERE name IN ('android_build_fingerprint', 'android_sdk_version', 'system_release', 'system_machine');
SELECT name, idx, severity, value FROM stats
WHERE value > 0 AND severity IN ('error', 'data_loss');

-- 2. Wall time storage. SUM(dur) = union hanya jika tidak ada overlap.
WITH ordered AS (
  SELECT *, MAX(ts + dur) OVER (
    ORDER BY ts, id ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING
  ) AS previous_end FROM wuwa_trace_calls
)
SELECT (b.end_ts - b.start_ts) / 1e9 AS trace_seconds,
  (MIN(c.ts) - b.start_ts) / 1e9 AS first_call_s,
  (MAX(c.ts + c.dur) - b.start_ts) / 1e9 AS last_call_end_s,
  (MAX(c.ts + c.dur) - MIN(c.ts)) / 1e9 AS storage_window_s,
  COUNT(*) AS calls, SUM(c.dur) / 1e9 AS call_sum_s,
  100.0 * SUM(c.dur) / (MAX(c.ts + c.dur) - MIN(c.ts)) AS occupancy_percent,
  SUM(CASE WHEN c.ts < c.previous_end THEN 1 ELSE 0 END) AS overlapping_calls
FROM ordered c CROSS JOIN trace_bounds b;

-- 3. Harus cocok satu-ke-satu; unknown harus nol sebelum menarik kesimpulan.
SELECT (SELECT COUNT(*) FROM wuwa_trace_calls) AS calls,
  (SELECT COUNT(*) FROM wuwa_trace_shells) AS direct_shells,
  COUNT(*) AS matched_rows, COUNT(DISTINCT id) AS matched_calls,
  SUM(CASE WHEN operation = 'unknown' THEN 1 ELSE 0 END) AS unknown_operations
FROM wuwa_trace_operations;
SELECT operation, COUNT(*) AS calls, SUM(dur) / 1e9 AS call_sum_s,
  MAX(dur) / 1e6 AS max_call_ms
FROM wuwa_trace_operations GROUP BY operation ORDER BY call_sum_s DESC;

-- 4. Materialisasi pohon proses sekali; hindari evaluasi ulang CTE rekursif.
-- Hitungan menyertakan UserService sendiri; kurangi satu untuk jumlah helper.
CREATE PERFETTO TABLE wuwa_trace_descendants AS
WITH RECURSIVE d(upid) AS (
  SELECT MAX(upid) FROM wuwa_trace_processes
  WHERE name = 'com.titotfp.wuwaid:wuwaid-storage'
  UNION ALL SELECT p.upid FROM process p JOIN d ON p.parent_upid = d.upid
)
SELECT p.upid, p.name FROM process p JOIN d USING (upid);
SELECT name, COUNT(*) AS processes FROM wuwa_trace_descendants
GROUP BY name ORDER BY processes DESC;
-- CPU seluruh trace, bukan wall time.
SELECT d.name, SUM(sc.dur) / 1e9 AS cpu_s FROM sched sc
JOIN thread t USING (utid) JOIN wuwa_trace_descendants d USING (upid)
WHERE sc.dur > 0 GROUP BY d.name ORDER BY cpu_s DESC;

-- 5. CPU app dalam window storage; SUM beberapa thread bukan wall time.
WITH w AS (SELECT MIN(ts) AS lo, MAX(ts + dur) AS hi FROM wuwa_trace_calls)
SELECT t.tid, t.name,
  SUM(MIN(sc.ts + sc.dur, w.hi) - MAX(sc.ts, w.lo)) / 1e9 AS cpu_s
FROM sched sc JOIN thread t USING (utid) CROSS JOIN w
WHERE t.upid = (
  SELECT MAX(upid) FROM wuwa_trace_processes WHERE name = 'com.titotfp.wuwaid'
) AND sc.dur > 0 AND sc.ts < w.hi AND sc.ts + sc.dur > w.lo
GROUP BY t.utid ORDER BY cpu_s DESC LIMIT 15;

-- 6. Panggilan paling lama, hash berulang, dan stat berulang.
SELECT id, (ts - b.start_ts) / 1e9 AS start_s, dur / 1e6 AS ms,
  operation, path_and_algorithm, tid, shell_pid
FROM wuwa_trace_operations CROSS JOIN trace_bounds b ORDER BY dur DESC LIMIT 15;
SELECT path_and_algorithm, COUNT(*) AS calls, SUM(dur) / 1e9 AS call_sum_s
FROM wuwa_trace_operations WHERE operation = 'hash'
GROUP BY path_and_algorithm ORDER BY calls DESC;
WITH numbered AS (
  SELECT *, ROW_NUMBER() OVER (PARTITION BY path_and_algorithm ORDER BY ts, id) AS n
  FROM wuwa_trace_operations WHERE operation = 'stat'
)
SELECT path_and_algorithm, COUNT(*) AS calls, SUM(dur) / 1e9 AS call_sum_s,
  SUM(CASE WHEN n > 1 THEN dur ELSE 0 END) / 1e9 AS repeat_sum_s
FROM numbered GROUP BY path_and_algorithm HAVING COUNT(*) > 1 ORDER BY calls DESC;

-- 7. Jeda antarcall (hanya valid untuk alur serial) dan mutasi sebagai anchor.
WITH ordered AS (
  SELECT *, LAG(ts + dur) OVER (ORDER BY ts, id) AS previous_end
  FROM wuwa_trace_operations
)
SELECT (previous_end - b.start_ts) / 1e9 AS gap_start_s,
  (ts - b.start_ts) / 1e9 AS gap_end_s, (ts - previous_end) / 1e9 AS gap_s,
  operation AS next_operation, path_and_algorithm
FROM ordered CROSS JOIN trace_bounds b
WHERE ts - previous_end > 100000000 ORDER BY gap_s DESC;
SELECT (ts - b.start_ts) / 1e9 AS start_s, dur / 1e6 AS ms,
  operation, path_and_algorithm
FROM wuwa_trace_operations CROSS JOIN trace_bounds b
WHERE operation IN ('force_stop', 'mkdir', 'write', 'copy', 'move', 'remove') ORDER BY ts;

-- 8. Lifecycle dan frame Android; bukan durasi operasi frontend/FPS WebView.
SELECT s.name, (s.ts - b.start_ts) / 1e9 AS start_s, s.dur / 1e6 AS ms
FROM slice s JOIN thread_track tt ON tt.id = s.track_id
JOIN thread t USING (utid) CROSS JOIN trace_bounds b
WHERE t.upid IN (
  SELECT upid FROM process WHERE cmdline IN ('com.titotfp.wuwaid', 'moe.shizuku.privileged.api')
) AND (s.name GLOB 'performResume:*' OR s.name GLOB 'performPause:*') ORDER BY s.ts;
SELECT jank_type, COUNT(*) AS frame_rows, MAX(dur) / 1e6 AS max_ms
FROM actual_frame_timeline_slice
WHERE upid = (
  SELECT MAX(upid) FROM wuwa_trace_processes WHERE name = 'com.titotfp.wuwaid'
) AND dur > 0 GROUP BY jank_type ORDER BY frame_rows DESC;

-- Cmdline mengandung quote/newline: ambil hex, jangan parse output mentah sebagai CSV.
SELECT pid, operation, HEX(cmdline) AS cmdline_hex
FROM wuwa_trace_shells WHERE operation = 'unknown';
