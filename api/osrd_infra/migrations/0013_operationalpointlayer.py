# Generated by Django 4.0.4 on 2022-05-19 10:21

import django.contrib.gis.db.models.fields
import django.db.models.deletion
from django.db import migrations, models


class Migration(migrations.Migration):

    dependencies = [
        ("osrd_infra", "0012_routelayer"),
    ]

    operations = [
        migrations.CreateModel(
            name="OperationalPointLayer",
            fields=[
                ("id", models.AutoField(auto_created=True, primary_key=True, serialize=False, verbose_name="ID")),
                ("obj_id", models.CharField(max_length=255)),
                ("geographic", django.contrib.gis.db.models.fields.MultiPointField(srid=3857)),
                ("schematic", django.contrib.gis.db.models.fields.MultiPointField(srid=3857)),
            ],
            options={
                "verbose_name_plural": "generated operational point layer",
            },
        ),
        migrations.RunSQL(
            """ALTER TABLE osrd_infra_operationalpointlayer
                ADD infra_id INTEGER,
                ADD CONSTRAINT osrd_infra_operationalpointlayer_fkey FOREIGN KEY (infra_id) REFERENCES osrd_infra_infra(id) ON DELETE CASCADE
            """,
            state_operations=[
                migrations.AddField(
                    model_name="operationalpointlayer",
                    name="infra",
                    field=models.ForeignKey(
                        on_delete=django.db.models.deletion.CASCADE,
                        to="osrd_infra.infra",
                    ),
                ),
            ],
        ),
        migrations.AlterUniqueTogether(
            name="operationalpointlayer",
            unique_together={("infra", "obj_id")},
        ),
    ]
