-- DO NOT EDIT THIS FILE MANUALLY!
-- To change the migration's content, use `editoast search make-migration`.
-- To add custom SQL code, check out `#[derive(Search)]` attributes `prepend_sql` and `append_sql`.

DROP TABLE IF EXISTS "search_signal";

CREATE TABLE "search_signal" (
    id BIGINT PRIMARY KEY REFERENCES "infra_object_signal"("id") ON UPDATE CASCADE ON DELETE CASCADE,
    "label" text,
    "line_name" text,
    "infra_id" integer,
    "obj_id" VARCHAR(255),
    "signaling_systems" TEXT[],
    "settings" TEXT[],
    "line_code" integer
);

CREATE INDEX "search_signal_label" ON "search_signal" USING gin ("label" gin_trgm_ops);
CREATE INDEX "search_signal_line_name" ON "search_signal" USING gin ("line_name" gin_trgm_ops);
CREATE INDEX "search_signal_infra_id" ON "search_signal" ("infra_id");
CREATE INDEX "search_signal_obj_id" ON "search_signal" ("obj_id");
CREATE INDEX "search_signal_signaling_systems" ON "search_signal" ("signaling_systems");
CREATE INDEX "search_signal_settings" ON "search_signal" ("settings");
CREATE INDEX "search_signal_line_code" ON "search_signal" ("line_code");

CREATE OR REPLACE FUNCTION search_signal__ins_trig_fun()
    RETURNS TRIGGER
    LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO "search_signal" (id, label, line_name, infra_id, obj_id, signaling_systems, settings, line_code)
        SELECT "infra_object_signal".id AS id, osrd_prepare_for_search(infra_object_signal.data->'extensions'->'sncf'->>'label') AS label,
    osrd_prepare_for_search(track_section.data->'extensions'->'sncf'->>'line_name') AS line_name,
    (infra_object_signal.infra_id) AS infra_id,
    (infra_object_signal.obj_id) AS obj_id,
    (ARRAY(SELECT jsonb_path_query(infra_object_signal.data, '$.logical_signals[*].signaling_system')->>0)) AS signaling_systems,
    (ARRAY(SELECT jsonb_path_query(infra_object_signal.data, '$.logical_signals[*].settings.keyvalue().key')->>0)) AS settings,
    ((track_section.data->'extensions'->'sncf'->>'line_code')::integer) AS line_code
        FROM (SELECT NEW.*) AS "infra_object_signal"
        
            INNER JOIN infra_object_track_section AS track_section
            ON track_section.infra_id = infra_object_signal.infra_id
                AND track_section.obj_id = infra_object_signal.data->>'track';
    RETURN NEW;
END;
$$;
CREATE OR REPLACE TRIGGER search_signal__ins_trig
AFTER INSERT ON "infra_object_signal"
FOR EACH ROW EXECUTE FUNCTION search_signal__ins_trig_fun();


CREATE OR REPLACE FUNCTION search_signal__upd_trig_fun()
    RETURNS TRIGGER
    LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE "search_signal"
        SET "label" = osrd_prepare_for_search(infra_object_signal.data->'extensions'->'sncf'->>'label'),
        "line_name" = osrd_prepare_for_search(track_section.data->'extensions'->'sncf'->>'line_name'),
        "infra_id" = (infra_object_signal.infra_id),
        "obj_id" = (infra_object_signal.obj_id),
        "signaling_systems" = (ARRAY(SELECT jsonb_path_query(infra_object_signal.data, '$.logical_signals[*].signaling_system')->>0)),
        "settings" = (ARRAY(SELECT jsonb_path_query(infra_object_signal.data, '$.logical_signals[*].settings.keyvalue().key')->>0)),
        "line_code" = ((track_section.data->'extensions'->'sncf'->>'line_code')::integer)
        FROM (SELECT NEW.*) AS "infra_object_signal"
        
            INNER JOIN infra_object_track_section AS track_section
            ON track_section.infra_id = infra_object_signal.infra_id
                AND track_section.obj_id = infra_object_signal.data->>'track'
        WHERE "infra_object_signal".id = "search_signal".id;
    RETURN NEW;
END;
$$;
CREATE OR REPLACE TRIGGER search_signal__upd_trig
AFTER UPDATE ON "infra_object_signal"
FOR EACH ROW EXECUTE FUNCTION search_signal__upd_trig_fun();



INSERT INTO "search_signal" (id, "label", "line_name", "infra_id", "obj_id", "signaling_systems", "settings", "line_code")
SELECT
    "infra_object_signal"."id" AS id,
    osrd_prepare_for_search(infra_object_signal.data->'extensions'->'sncf'->>'label') AS label
,    osrd_prepare_for_search(track_section.data->'extensions'->'sncf'->>'line_name') AS line_name
,    (infra_object_signal.infra_id) AS infra_id
,    (infra_object_signal.obj_id) AS obj_id
,    (ARRAY(SELECT jsonb_path_query(infra_object_signal.data, '$.logical_signals[*].signaling_system')->>0)) AS signaling_systems
,    (ARRAY(SELECT jsonb_path_query(infra_object_signal.data, '$.logical_signals[*].settings.keyvalue().key')->>0)) AS settings
,    ((track_section.data->'extensions'->'sncf'->>'line_code')::integer) AS line_code
FROM "infra_object_signal"
    
            INNER JOIN infra_object_track_section AS track_section
            ON track_section.infra_id = infra_object_signal.infra_id
                AND track_section.obj_id = infra_object_signal.data->>'track';
