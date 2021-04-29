# Generated by Django 3.1.7 on 2021-04-27 09:45

import django.contrib.gis.db.models.fields
from django.db import migrations, models
import django.db.models.deletion


class Migration(migrations.Migration):

    initial = True

    dependencies = [
        ('contenttypes', '0002_remove_content_type_name'),
    ]

    operations = [
        migrations.CreateModel(
            name='Entity',
            fields=[
                ('entity_id', models.BigAutoField(primary_key=True, serialize=False)),
            ],
        ),
        migrations.CreateModel(
            name='EntityNamespace',
            fields=[
                ('id', models.AutoField(auto_created=True, primary_key=True, serialize=False, verbose_name='ID')),
            ],
        ),
        migrations.CreateModel(
            name='IdentifierComponent',
            fields=[
                ('component_id', models.BigAutoField(primary_key=True, serialize=False)),
                ('name', models.CharField(max_length=255)),
            ],
            options={
                'default_related_name': 'identifier',
            },
        ),
        migrations.CreateModel(
            name='IdentifierDatabase',
            fields=[
                ('id', models.AutoField(auto_created=True, primary_key=True, serialize=False, verbose_name='ID')),
                ('name', models.CharField(max_length=255)),
            ],
        ),
        migrations.CreateModel(
            name='TrackSectionRangeComponent',
            fields=[
                ('component_id', models.BigAutoField(primary_key=True, serialize=False)),
                ('start_offset', models.FloatField()),
                ('end_offset', models.FloatField()),
                ('entity', models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='range_location', to='osrd_infra.entity')),
            ],
            options={
                'default_related_name': 'range_location',
            },
        ),
        migrations.CreateModel(
            name='TrackSectionLocationComponent',
            fields=[
                ('component_id', models.BigAutoField(primary_key=True, serialize=False)),
                ('offset', models.FloatField()),
                ('entity', models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='point_location', to='osrd_infra.entity')),
            ],
            options={
                'default_related_name': 'point_location',
            },
        ),
        migrations.CreateModel(
            name='TrackSectionLinkComponent',
            fields=[
                ('component_id', models.BigAutoField(primary_key=True, serialize=False)),
                ('begin_endpoint', models.IntegerField(choices=[(0, 'Begin'), (1, 'End')])),
                ('end_endpoint', models.IntegerField(choices=[(0, 'Begin'), (1, 'End')])),
                ('entity', models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='track_section_link', to='osrd_infra.entity')),
            ],
            options={
                'default_related_name': 'track_section_link',
            },
        ),
        migrations.CreateModel(
            name='TrackSectionComponent',
            fields=[
                ('component_id', models.BigAutoField(primary_key=True, serialize=False)),
                ('path', django.contrib.gis.db.models.fields.LineStringField(srid=3857)),
                ('length', models.FloatField()),
                ('entity', models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='track_section', to='osrd_infra.entity')),
            ],
            options={
                'default_related_name': 'track_section',
            },
        ),
        migrations.CreateModel(
            name='Infra',
            fields=[
                ('id', models.AutoField(auto_created=True, primary_key=True, serialize=False, verbose_name='ID')),
                ('name', models.CharField(max_length=128)),
                ('owner', models.UUIDField(default='00000000-0000-0000-0000-000000000000')),
                ('created', models.DateTimeField(auto_now_add=True)),
                ('modified', models.DateTimeField(auto_now=True)),
                ('namespace', models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, to='osrd_infra.entitynamespace')),
            ],
        ),
        migrations.AddConstraint(
            model_name='identifierdatabase',
            constraint=models.UniqueConstraint(fields=('name',), name='identifier_database_unique_name'),
        ),
        migrations.AddField(
            model_name='identifiercomponent',
            name='database',
            field=models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='identifier', to='osrd_infra.identifierdatabase'),
        ),
        migrations.AddField(
            model_name='identifiercomponent',
            name='entity',
            field=models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='identifier', to='osrd_infra.entity'),
        ),
        migrations.AddField(
            model_name='entity',
            name='entity_type',
            field=models.ForeignKey(editable=False, on_delete=django.db.models.deletion.CASCADE, to='contenttypes.contenttype'),
        ),
        migrations.AddField(
            model_name='entity',
            name='namespace',
            field=models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, to='osrd_infra.entitynamespace'),
        ),
        migrations.CreateModel(
            name='OperationalPointEntity',
            fields=[
            ],
            options={
                'verbose_name_plural': 'operational point entities',
                'proxy': True,
                'indexes': [],
                'constraints': [],
            },
            bases=('osrd_infra.entity',),
        ),
        migrations.CreateModel(
            name='SignalEntity',
            fields=[
            ],
            options={
                'verbose_name_plural': 'signal entities',
                'proxy': True,
                'indexes': [],
                'constraints': [],
            },
            bases=('osrd_infra.entity',),
        ),
        migrations.CreateModel(
            name='SwitchEntity',
            fields=[
            ],
            options={
                'verbose_name_plural': 'switch entities',
                'proxy': True,
                'indexes': [],
                'constraints': [],
            },
            bases=('osrd_infra.entity',),
        ),
        migrations.CreateModel(
            name='TrackSectionEntity',
            fields=[
            ],
            options={
                'verbose_name_plural': 'track section entities',
                'proxy': True,
                'indexes': [],
                'constraints': [],
            },
            bases=('osrd_infra.entity',),
        ),
        migrations.CreateModel(
            name='TrackSectionLinkEntity',
            fields=[
            ],
            options={
                'verbose_name_plural': 'track section link entities',
                'proxy': True,
                'indexes': [],
                'constraints': [],
            },
            bases=('osrd_infra.entity',),
        ),
        migrations.AddField(
            model_name='tracksectionrangecomponent',
            name='track_section',
            field=models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='range_objects', to='osrd_infra.tracksectionentity'),
        ),
        migrations.AddField(
            model_name='tracksectionlocationcomponent',
            name='track_section',
            field=models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='point_objects', to='osrd_infra.tracksectionentity'),
        ),
        migrations.AddField(
            model_name='tracksectionlinkcomponent',
            name='begin_track_section',
            field=models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='link_begin_branches', to='osrd_infra.tracksectionentity'),
        ),
        migrations.AddField(
            model_name='tracksectionlinkcomponent',
            name='end_track_section',
            field=models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='link_end_branches', to='osrd_infra.tracksectionentity'),
        ),
        migrations.AddConstraint(
            model_name='identifiercomponent',
            constraint=models.UniqueConstraint(fields=('database', 'name'), name='identifier_unique_in_database'),
        ),
    ]
