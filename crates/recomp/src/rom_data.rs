//! Source-authenticated extraction, preparation and verified dataset loading.
use gba_core::rom_data::{
    metadata16::Metadata16, METADATA16_ID, METADATA16_FILE, MAX_METADATA16_FILE_BYTES,
    metadata52::Metadata52, METADATA52_ID, METADATA52_FILE, MAX_METADATA52_FILE_BYTES,
    metadata4::Metadata4, METADATA4_ID, METADATA4_FILE, MAX_METADATA4_FILE_BYTES,
    metadata2::Metadata2, METADATA2_ID, METADATA2_FILE, MAX_METADATA2_FILE_BYTES,
    default48::Template as Default48, DEFAULT48_ID, DEFAULT48_FILE, MAX_DEFAULT48_FILE_BYTES,
    constructor_alternates::Alternates as ConstructorAlternates, CONSTRUCTOR_ALTERNATES_ID, CONSTRUCTOR_ALTERNATES_FILE, MAX_CONSTRUCTOR_ALTERNATES_FILE_BYTES,
    metadata14::Metadata14, METADATA14_ID, METADATA14_FILE, MAX_METADATA14_FILE_BYTES,
    metadata12::Metadata12, METADATA12_ID, METADATA12_FILE, MAX_METADATA12_FILE_BYTES,
    global_pair_flags::GlobalPairFlags, GLOBAL_PAIR_FLAGS_ID, GLOBAL_PAIR_FLAGS_FILE, MAX_GLOBAL_PAIR_FLAGS_FILE_BYTES,
    evaluation_parameters::EvaluationParameters, EVALUATION_PARAMETERS_ID, EVALUATION_PARAMETERS_FILE, MAX_EVALUATION_PARAMETERS_FILE_BYTES,
    progression_rewards::ProgressionRewards, PROGRESSION_REWARDS_ID, PROGRESSION_REWARDS_FILE, MAX_PROGRESSION_REWARDS_FILE_BYTES,
    progression_gates::ProgressionGates, PROGRESSION_GATES_ID, PROGRESSION_GATES_FILE, MAX_PROGRESSION_GATES_FILE_BYTES,
    progression_selection_gates::ProgressionSelectionGates, PROGRESSION_SELECTION_GATES_ID, PROGRESSION_SELECTION_GATES_FILE, MAX_PROGRESSION_SELECTION_GATES_FILE_BYTES,
    aggregate_flag_keys::AggregateFlagKeys, AGGREGATE_FLAG_KEYS_ID, AGGREGATE_FLAG_KEYS_FILE, MAX_AGGREGATE_FLAG_KEYS_FILE_BYTES,
    trade_parameters::TradeParameters, TRADE_PARAMETERS_ID, TRADE_PARAMETERS_FILE, MAX_TRADE_PARAMETERS_FILE_BYTES,
    item_effect_parameters::ItemEffectParameters, ITEM_EFFECT_PARAMETERS_ID, ITEM_EFFECT_PARAMETERS_FILE, MAX_ITEM_EFFECT_PARAMETERS_FILE_BYTES,
    audio_directory::AudioDirectory, AUDIO_DIRECTORY_ID, AUDIO_DIRECTORY_FILE, MAX_AUDIO_DIRECTORY_FILE_BYTES,
    audio_tones::AudioTones, AUDIO_TONES_ID, AUDIO_TONES_FILE, MAX_AUDIO_TONES_FILE_BYTES,
    audio_sequences::AudioSequences, AUDIO_SEQUENCES_ID, AUDIO_SEQUENCES_FILE, MAX_AUDIO_SEQUENCES_FILE_BYTES,
    audio_driver::AudioDriver, AUDIO_DRIVER_ID, AUDIO_DRIVER_FILE, MAX_AUDIO_DRIVER_FILE_BYTES,
    palette_effect_dispatch::PaletteEffectDispatch, PALETTE_EFFECT_DISPATCH_ID, PALETTE_EFFECT_DISPATCH_FILE, MAX_PALETTE_EFFECT_DISPATCH_FILE_BYTES,
    palette_programs::PalettePrograms, PALETTE_PROGRAMS_ID, PALETTE_PROGRAMS_FILE, MAX_PALETTE_PROGRAMS_FILE_BYTES,
    blend_programs::BlendPrograms, BLEND_PROGRAMS_ID, BLEND_PROGRAMS_FILE, MAX_BLEND_PROGRAMS_FILE_BYTES,
    task_descriptors::TaskDescriptors, TASK_DESCRIPTORS_ID, TASK_DESCRIPTORS_FILE, MAX_TASK_DESCRIPTORS_FILE_BYTES,
    effect_event_directories::EffectEventDirectories, EFFECT_EVENT_DIRECTORIES_ID, EFFECT_EVENT_DIRECTORIES_FILE, MAX_EFFECT_EVENT_DIRECTORIES_FILE_BYTES,
    script_commands::ScriptCommands, SCRIPT_COMMANDS_ID, SCRIPT_COMMANDS_FILE, MAX_SCRIPT_COMMANDS_FILE_BYTES,
    script_archive::ScriptArchive, SCRIPT_ARCHIVE_ID, SCRIPT_ARCHIVE_FILE, MAX_SCRIPT_ARCHIVE_FILE_BYTES,
    script_text::ScriptText, SCRIPT_TEXT_ID, SCRIPT_TEXT_FILE, MAX_SCRIPT_TEXT_FILE_BYTES,
    script_coordinates::ScriptCoordinates, SCRIPT_COORDINATES_ID, SCRIPT_COORDINATES_FILE, MAX_SCRIPT_COORDINATES_FILE_BYTES,
    ui_choice_lists::UiChoiceLists, UI_CHOICE_LISTS_ID, UI_CHOICE_LISTS_FILE, MAX_UI_CHOICE_LISTS_FILE_BYTES,
    unit_obj_initial_pose::UnitObjInitialPose, UNIT_OBJ_INITIAL_POSE_ID, UNIT_OBJ_INITIAL_POSE_FILE, MAX_UNIT_OBJ_INITIAL_POSE_FILE_BYTES,
    color_mode_palettes::ColorModePalettes, COLOR_MODE_PALETTES_ID, COLOR_MODE_PALETTES_FILE, MAX_COLOR_MODE_PALETTES_FILE_BYTES,
    unit_obj_equipment::UnitObjEquipment, UNIT_OBJ_EQUIPMENT_ID, UNIT_OBJ_EQUIPMENT_FILE, MAX_UNIT_OBJ_EQUIPMENT_FILE_BYTES,
    unit_obj_normal_battle::UnitObjNormalBattle, UNIT_OBJ_NORMAL_BATTLE_ID, UNIT_OBJ_NORMAL_BATTLE_FILE, MAX_UNIT_OBJ_NORMAL_BATTLE_FILE_BYTES,
    unit_obj_alternate_battle::UnitObjAlternateBattle, UNIT_OBJ_ALTERNATE_BATTLE_ID, UNIT_OBJ_ALTERNATE_BATTLE_FILE, MAX_UNIT_OBJ_ALTERNATE_BATTLE_FILE_BYTES,
    unit_obj_script_special_normal::UnitObjScriptSpecialNormal, UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_ID, UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FILE, MAX_UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FILE_BYTES,
    unit_obj_script_special_alternate::UnitObjScriptSpecialAlternate, UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_ID, UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FILE, MAX_UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FILE_BYTES,
    unit_obj_script_override::UnitObjScriptOverride, UNIT_OBJ_SCRIPT_OVERRIDE_ID, UNIT_OBJ_SCRIPT_OVERRIDE_FILE, MAX_UNIT_OBJ_SCRIPT_OVERRIDE_FILE_BYTES,
    unit_obj_script_registry_special::UnitObjScriptRegistrySpecial, UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_ID, UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FILE, MAX_UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FILE_BYTES,
    unit_obj_script_registry_generic::UnitObjScriptRegistryGeneric, UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_ID, UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FILE, MAX_UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FILE_BYTES,
    unit_obj_property_words::UnitObjPropertyWords, UNIT_OBJ_PROPERTY_WORDS_ID, UNIT_OBJ_PROPERTY_WORDS_FILE, MAX_UNIT_OBJ_PROPERTY_WORDS_FILE_BYTES,
    unit_obj_byte_property_words::UnitObjBytePropertyWords, UNIT_OBJ_BYTE_PROPERTY_WORDS_ID, UNIT_OBJ_BYTE_PROPERTY_WORDS_FILE, MAX_UNIT_OBJ_BYTE_PROPERTY_WORDS_FILE_BYTES,
    unit_obj_ffff_reader_fields::UnitObjFfffReaderFields, UNIT_OBJ_FFFF_READER_FIELDS_ID, UNIT_OBJ_FFFF_READER_FIELDS_FILE, MAX_UNIT_OBJ_FFFF_READER_FIELDS_FILE_BYTES,
    slot_job_lookup::SlotJobLookup, SLOT_JOB_LOOKUP_ID, SLOT_JOB_LOOKUP_FILE, MAX_SLOT_JOB_LOOKUP_FILE_BYTES,
    observed_task_parameters::ObservedTaskParameters, OBSERVED_TASK_PARAMETERS_ID, OBSERVED_TASK_PARAMETERS_FILE, MAX_OBSERVED_TASK_PARAMETERS_FILE_BYTES,
    obj_lists::constructor_animation::Animation as ConstructorObjAnimation, CONSTRUCTOR_OBJ_ANIMATION_ID, CONSTRUCTOR_OBJ_ANIMATION_FILE, MAX_CONSTRUCTOR_OBJ_ANIMATION_FILE_BYTES,
    task_cycle_deltas::TaskCycleDeltas, TASK_CYCLE_DELTAS_ID, TASK_CYCLE_DELTAS_FILE, MAX_TASK_CYCLE_DELTAS_FILE_BYTES,
    ui_raw_transfers::RawTransfers, UI_RAW_TRANSFERS_ID, UI_RAW_TRANSFERS_FILE, MAX_UI_RAW_TRANSFERS_FILE_BYTES,
    ui_raw_transfers::dynamic_fragments::Fragments as DynamicRawFragments, DYNAMIC_RAW_FRAGMENTS_ID, DYNAMIC_RAW_FRAGMENTS_FILE, MAX_DYNAMIC_RAW_FRAGMENTS_FILE_BYTES,
    conditional_four_byte_lookup::Lookup as ConditionalFourByteLookup, CONDITIONAL_FOUR_BYTE_LOOKUP_ID, CONDITIONAL_FOUR_BYTE_LOOKUP_FILE, MAX_CONDITIONAL_FOUR_BYTE_LOOKUP_FILE_BYTES,
    conditional_packed_lookup::Lookup as ConditionalPackedLookup, CONDITIONAL_PACKED_LOOKUP_ID, CONDITIONAL_PACKED_LOOKUP_FILE, MAX_CONDITIONAL_PACKED_LOOKUP_FILE_BYTES,
    conditional_descriptor_banks::DescriptorBanks as ConditionalDescriptorBanks, CONDITIONAL_DESCRIPTOR_BANKS_ID, CONDITIONAL_DESCRIPTOR_BANKS_FILE, MAX_CONDITIONAL_DESCRIPTOR_BANKS_FILE_BYTES,
    conditional_descriptor_bank_zero::BankZero as ConditionalDescriptorBankZero, CONDITIONAL_DESCRIPTOR_BANK_ZERO_ID, CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE, MAX_CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE_BYTES,
    conditional_property_masks::Masks as ConditionalPropertyMasks, CONDITIONAL_PROPERTY_MASKS_ID, CONDITIONAL_PROPERTY_MASKS_FILE, MAX_CONDITIONAL_PROPERTY_MASKS_FILE_BYTES,
    compact_obj_animations::CompactAnimations as CompactObjAnimations, COMPACT_OBJ_ANIMATIONS_ID, COMPACT_OBJ_ANIMATIONS_FILE, MAX_COMPACT_OBJ_ANIMATIONS_FILE_BYTES,
    compact_obj_animations_secondary::CompactAnimationsSecondary as SecondaryCompactObjAnimations, SECONDARY_COMPACT_OBJ_ANIMATIONS_ID, SECONDARY_COMPACT_OBJ_ANIMATIONS_FILE, MAX_SECONDARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES,
    compact_obj_animations_tertiary::CompactAnimationsTertiary as TertiaryCompactObjAnimations, TERTIARY_COMPACT_OBJ_ANIMATIONS_ID, TERTIARY_COMPACT_OBJ_ANIMATIONS_FILE, MAX_TERTIARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES,
    compact_obj_animations_quaternary::CompactAnimationsQuaternary as QuaternaryCompactObjAnimations, QUATERNARY_COMPACT_OBJ_ANIMATIONS_ID, QUATERNARY_COMPACT_OBJ_ANIMATIONS_FILE, MAX_QUATERNARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES,
    compact_obj_animations_quinary::CompactAnimationsQuinary as QuinaryCompactObjAnimations, QUINARY_COMPACT_OBJ_ANIMATIONS_ID, QUINARY_COMPACT_OBJ_ANIMATIONS_FILE, MAX_QUINARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES,
    compact_obj_animations_senary::CompactAnimationsSenary as SenaryCompactObjAnimations, SENARY_COMPACT_OBJ_ANIMATIONS_ID, SENARY_COMPACT_OBJ_ANIMATIONS_FILE, MAX_SENARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES,
    indexed_graphics::IndexedGraphics, INDEXED_GRAPHICS_ID, INDEXED_GRAPHICS_FILE, MAX_INDEXED_GRAPHICS_FILE_BYTES,
    world_records38::WorldRecords38, WORLD_RECORDS38_ID, WORLD_RECORDS38_FILE, MAX_WORLD_RECORDS38_FILE_BYTES,
    parameter_records27::ParameterRecords27, PARAMETER_RECORDS27_ID, PARAMETER_RECORDS27_FILE, MAX_PARAMETER_RECORDS27_FILE_BYTES,
    event_records12::EventRecords12, EVENT_RECORDS12_ID, EVENT_RECORDS12_FILE, MAX_EVENT_RECORDS12_FILE_BYTES,
    selection_byte_tables::ByteTables, SELECTION_BYTE_TABLES_ID, SELECTION_BYTE_TABLES_FILE, MAX_SELECTION_BYTE_TABLES_FILE_BYTES,
    mission_records70::Records70, MISSION_RECORDS70_ID, MISSION_RECORDS70_FILE, MAX_MISSION_RECORDS70_FILE_BYTES,
    conditional_records8::Records8, CONDITIONAL_RECORDS8_ID, CONDITIONAL_RECORDS8_FILE, MAX_CONDITIONAL_RECORDS8_FILE_BYTES,
    records4::Records4, RECORDS4_ID, RECORDS4_FILE, MAX_RECORDS4_FILE_BYTES,
    metadata28::Metadata28, METADATA28_ID, METADATA28_FILE, MAX_METADATA28_FILE_BYTES,
    encounter_coordinates::EncounterCoordinates, ENCOUNTER_COORDINATES_ID, ENCOUNTER_COORDINATES_FILE, MAX_ENCOUNTER_COORDINATES_FILE_BYTES,
    encounter_unit_templates::EncounterUnitTemplates, ENCOUNTER_UNIT_TEMPLATES_ID, ENCOUNTER_UNIT_TEMPLATES_FILE, MAX_ENCOUNTER_UNIT_TEMPLATES_FILE_BYTES,
    encounter_headers::EncounterHeaders, ENCOUNTER_HEADERS_ID, ENCOUNTER_HEADERS_FILE, MAX_ENCOUNTER_HEADERS_FILE_BYTES,
    world_roads::WorldRoads, WORLD_ROADS_ID, WORLD_ROADS_FILE, MAX_WORLD_ROADS_FILE_BYTES,
    world_nodes::WorldNodes, WORLD_NODES_ID, WORLD_NODES_FILE, MAX_WORLD_NODES_FILE_BYTES,
    word_task_obj_animations::WordTaskObjAnimations, WORD_TASK_OBJ_ANIMATIONS_ID, WORD_TASK_OBJ_ANIMATIONS_FILE, MAX_WORD_TASK_OBJ_ANIMATIONS_FILE_BYTES,
    map_raster_palettes::MapRasterPalettes, MAP_RASTER_PALETTES_ID, MAP_RASTER_PALETTES_FILE, MAX_MAP_RASTER_PALETTES_FILE_BYTES,
    auxiliary_scripts::AuxiliaryCommands, AUXILIARY_COMMANDS_ID, AUXILIARY_COMMANDS_FILE, MAX_AUXILIARY_COMMANDS_FILE_BYTES,
    auxiliary_scripts::AuxiliaryArchive, AUXILIARY_ARCHIVE_ID, AUXILIARY_ARCHIVE_FILE, MAX_AUXILIARY_ARCHIVE_FILE_BYTES,
    item_links::Links as ItemLinks, ITEM_LINKS_ID, ITEM_LINKS_FILE, MAX_ITEM_LINKS_FILE_BYTES,
    items::Items, ITEM_RECORDS_ID, ITEM_RECORDS_FILE, MAX_ITEM_RECORDS_FILE_BYTES,
    ui_graphics::Graphics as UiGraphics, UI_CUSTOM_GRAPHICS_ID, UI_CUSTOM_GRAPHICS_FILE, MAX_UI_CUSTOM_GRAPHICS_FILE_BYTES,
    ui_palettes::Palettes as UiPalettes, UI_RAW_PALETTES_ID, UI_RAW_PALETTES_FILE, MAX_UI_RAW_PALETTES_FILE_BYTES,
    obj_graphics::Graphics as ObjGraphics, OBJ_CUSTOM_GRAPHICS_ID, OBJ_CUSTOM_GRAPHICS_FILE, MAX_OBJ_CUSTOM_GRAPHICS_FILE_BYTES,
    obj_animations::Banks as ObjAnimationBanks, OBJ_ANIMATION_BANKS_ID, OBJ_ANIMATION_BANKS_FILE, MAX_OBJ_ANIMATION_BANKS_FILE_BYTES,
    obj_lists::Lists as ObjLists, OBJ_STATIC_LISTS_ID, OBJ_STATIC_LISTS_FILE, MAX_OBJ_STATIC_LISTS_FILE_BYTES,
    fonts::FontBank, FONT_BANK_ID, FONT_BANK_FILE, MAX_FONT_BANK_FILE_BYTES,
    name_keyboard::Keyboard, NAME_KEYBOARD_ID, NAME_KEYBOARD_FILE, MAX_NAME_KEYBOARD_FILE_BYTES,
    bg_descriptors::Descriptors, bg_palettes::Palettes, BaseFiles, Extraction, Manifest,
    ObjDimensions, BG_BASE_PALETTES_FILE, BG_BASE_PALETTES_ID, BG_DESCRIPTORS_FILE,
    BG_DESCRIPTORS_ID, MANIFEST_FILE, MAX_BG_BASE_PALETTES_FILE_BYTES,
    MAX_BG_DESCRIPTORS_FILE_BYTES, MAX_DATASET_BYTES, MAX_FILE_BYTES, MAX_MANIFEST_FILE_BYTES,
    OBJ_DIMENSIONS_FILE,
    bg_alternate_palettes::Palettes as AlternatePalettes,
    BG_ALTERNATE_PALETTES_FILE, BG_ALTERNATE_PALETTES_ID,
    MAX_BG_ALTERNATE_PALETTES_FILE_BYTES,
    bg_packet_data::Programs as PacketPrograms,
    BG_PACKET_PROGRAMS_ID, BG_PACKET_PROGRAMS_FILE, MAX_BG_PACKET_PROGRAMS_FILE_BYTES,
    bg_graphics::Graphics, BG_MAIN_GRAPHICS_ID, BG_MAIN_GRAPHICS_FILE,
    MAX_BG_MAIN_GRAPHICS_FILE_BYTES,
    bg_transfers::Transfers, BG_TRANSFER_PROGRAMS_ID, BG_TRANSFER_PROGRAMS_FILE,
    MAX_BG_TRANSFER_PROGRAMS_FILE_BYTES,
    text_short::ShortTexts, SHORT_TEXTS_ID, SHORT_TEXTS_FILE, MAX_SHORT_TEXTS_FILE_BYTES,
    text_catalogs::TextCatalogs, TEXT_CATALOGS_ID, TEXT_CATALOGS_FILE, MAX_TEXT_CATALOGS_FILE_BYTES,
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[cfg(test)]
mod alternate_palette_tests;
#[cfg(test)]
mod alternate_palette_data_tests;
#[cfg(test)]
mod alternate_palette_runtime_tests;
#[cfg(test)]
mod bg_descriptor_tests;
#[cfg(test)]
mod bg_graphics_tests;
#[cfg(test)]
mod bg_graphics_managed_tests;
#[cfg(test)]
mod bg_graphics_runtime_tests;
#[cfg(test)]
mod bg_transfer_tests;
#[cfg(test)]
mod bg_transfer_managed_tests;
#[cfg(test)]
mod bg_transfer_runtime_tests;
#[cfg(test)]
mod text_short_tests;
#[cfg(test)]
mod text_short_managed_tests;
#[cfg(test)]
mod text_short_runtime_tests;
#[cfg(test)]
mod text_catalog_tests;
#[cfg(test)]
mod text_catalog_managed_tests;
#[cfg(test)]
mod text_catalog_runtime_tests;
#[cfg(test)]
mod font_keyboard_tests;
#[cfg(test)]
mod obj_list_tests;
#[cfg(test)]
mod obj_list_runtime_tests;
#[cfg(test)]
mod obj_extra_tests;
#[cfg(test)]
mod obj_ui_property_tests;
#[cfg(test)]
mod observed_obj_lists_model_tests;
#[cfg(test)]
mod constructor_obj_animation_model_tests;
#[cfg(test)]
mod constructor_obj_animation_runtime_tests;
#[cfg(test)]
mod constructor_obj_animation_scene_tests;
#[cfg(test)]
mod observed_obj_lists_runtime_tests;
#[cfg(test)]
mod obj_animation_tests;
#[cfg(test)]
mod obj_zero_count_tests;
#[cfg(test)]
mod obj_zero_count_runtime_tests;
#[cfg(test)]
mod obj_localized_tests;
#[cfg(test)]
mod obj_localized_runtime_tests;
#[cfg(test)]
mod obj_graphics_tests;
#[cfg(test)]
mod ui_palette_tests;
#[cfg(test)]
mod ui_graphics_tests;
#[cfg(test)]
mod item_tests;
#[cfg(test)]
mod item_link_tests;
#[cfg(test)]
mod metadata16_tests;
#[cfg(test)]
mod metadata52_tests;
#[cfg(test)]
mod metadata4_tests;
#[cfg(test)]
mod metadata2_tests;
#[cfg(test)]
mod default48_tests;
#[cfg(test)]
mod constructor_alternate_tests;
#[cfg(test)]
mod metadata14_tests;
#[cfg(test)]
mod metadata12_tests;
#[cfg(test)]
mod global_pair_flags_tests;
#[cfg(test)]
mod evaluation_parameter_tests;
#[cfg(test)]
mod progression_reward_tests;
#[cfg(test)]
mod progression_gate_tests;
#[cfg(test)]
mod progression_selection_gate_tests;
#[cfg(test)]
mod aggregate_flag_key_tests;
#[cfg(test)]
mod trade_parameter_tests;
#[cfg(test)]
mod item_effect_parameter_tests;
#[cfg(test)]
mod audio_directory_tests;
#[cfg(test)]
mod audio_tone_source_tests;
#[cfg(test)]
mod audio_tone_model_tests;
#[cfg(test)]
mod audio_sequence_model_tests;
#[cfg(test)]
mod audio_driver_model_tests;
#[cfg(test)]
mod palette_effect_dispatch_model_tests;
#[cfg(test)]
mod palette_program_model_tests;
#[cfg(test)]
mod blend_program_model_tests;
#[cfg(test)]
mod task_descriptor_model_tests;
#[cfg(test)]
mod effect_event_directory_model_tests;
#[cfg(test)]
mod script_command_model_tests;
#[cfg(test)]
mod script_archive_model_tests;
#[cfg(test)]
mod script_obj_query_tests;
#[cfg(test)]
mod script_text_model_tests;
#[cfg(test)]
mod script_coordinates_model_tests;
#[cfg(test)]
mod auxiliary_scripts_model_tests;
#[cfg(test)]
mod map_raster_palette_model_tests;
#[cfg(test)]
mod ui_choice_list_model_tests;
#[cfg(test)]
mod unit_obj_initial_pose_model_tests;
#[cfg(test)]
mod unit_obj_equipment_model_tests;
#[cfg(test)]
mod unit_obj_normal_battle_model_tests;
#[cfg(test)]
mod script_special_normal_model_tests;
#[cfg(test)]
mod script_special_alternate_model_tests;
#[cfg(test)]
mod script_override_model_tests;
#[cfg(test)]
mod script_registry_special_model_tests;
#[cfg(test)]
mod script_registry_generic_model_tests;
#[cfg(test)]
mod unit_obj_property_words_model_tests;
#[cfg(test)]
mod unit_obj_byte_property_words_model_tests;
#[cfg(test)]
mod unit_obj_ffff_reader_fields_model_tests;
#[cfg(test)]
mod slot_job_lookup_model_tests;
#[cfg(test)]
mod observed_task_parameters_model_tests;
#[cfg(test)]
mod task_cycle_deltas_model_tests;
#[cfg(test)]
mod task_cycle_deltas_runtime_tests;
#[cfg(test)]
mod metadata28_scene_tests;
#[cfg(test)]
mod conditional_records8_scene_tests;
#[cfg(test)]
mod mission_records70_scene_tests;
#[cfg(test)]
mod selection_byte_tables_scene_tests;
#[cfg(test)]
mod event_records12_scene_tests;
#[cfg(test)]
mod world_records38_scene_tests;
#[cfg(test)]
mod parameter_records27_scene_tests;
#[cfg(test)]
mod indexed_graphics_scene_tests;
#[cfg(test)]
mod records4_scene_tests;
#[cfg(test)]
mod metadata28_runtime_tests;
#[cfg(test)]
mod records4_runtime_tests;
#[cfg(test)]
mod conditional_records8_runtime_tests;
#[cfg(test)]
mod mission_records70_runtime_tests;
#[cfg(test)]
mod selection_byte_tables_runtime_tests;
#[cfg(test)]
mod event_records12_runtime_tests;
#[cfg(test)]
mod parameter_records27_runtime_tests;
#[cfg(test)]
mod world_records38_runtime_tests;
#[cfg(test)]
mod indexed_graphics_runtime_tests;
#[cfg(test)]
mod ui_raw_transfers_runtime_tests;
#[cfg(test)]
mod dynamic_raw_fragments_runtime_tests;
#[cfg(test)]
mod clan_table_alias_runtime_tests;
#[cfg(test)]
mod clan_table_alias_native_tests;
#[cfg(test)]
mod conditional_four_byte_lookup_tests;
#[cfg(test)]
mod conditional_packed_lookup_tests;
#[cfg(test)]
mod conditional_descriptor_banks_tests;
#[cfg(test)]
mod conditional_descriptor_bank_zero_tests;
#[cfg(test)]
mod conditional_property_masks_tests;
#[cfg(test)]
mod compact_obj_animations_tests;
#[cfg(test)]
mod compact_obj_animations_secondary_tests;
#[cfg(test)]
mod compact_obj_animations_tertiary_tests;
#[cfg(test)]
mod compact_obj_animations_quaternary_tests;
#[cfg(test)]
mod compact_obj_animations_quinary_tests;
#[cfg(test)]
mod compact_obj_animations_senary_tests;
#[cfg(test)]
mod compact_obj_animations_tertiary_runtime_tests;
#[cfg(test)]
mod compact_obj_animations_quaternary_runtime_tests;
#[cfg(test)]
mod compact_obj_animations_quinary_runtime_tests;
#[cfg(test)]
mod compact_obj_animations_senary_runtime_tests;
#[cfg(test)]
mod compact_obj_animations_secondary_runtime_tests;
#[cfg(test)]
mod compact_obj_animations_runtime_tests;
#[cfg(test)]
mod conditional_property_masks_runtime_tests;
#[cfg(test)]
mod conditional_descriptor_bank_zero_runtime_tests;
#[cfg(test)]
mod conditional_descriptor_banks_runtime_tests;
#[cfg(test)]
mod conditional_packed_lookup_runtime_tests;
#[cfg(test)]
mod conditional_four_byte_lookup_runtime_tests;
#[cfg(test)]
mod encounter_coordinate_runtime_tests;
#[cfg(test)]
mod encounter_unit_template_runtime_tests;
#[cfg(test)]
mod encounter_header_runtime_tests;
#[cfg(test)]
mod world_road_runtime_tests;
#[cfg(test)]
mod world_node_runtime_tests;
#[cfg(test)]
mod word_task_obj_runtime_tests;
#[cfg(test)]
mod unit_obj_alternate_battle_model_tests;
#[cfg(test)]
mod color_mode_palette_model_tests;
#[cfg(test)]
mod metadata4_runtime_tests;
#[cfg(test)]
mod metadata2_runtime_tests;
#[cfg(test)]
mod default48_runtime_tests;
#[cfg(test)]
mod constructor_alternate_runtime_tests;
#[cfg(test)]
mod metadata14_runtime_tests;
#[cfg(test)]
mod metadata12_runtime_tests;
#[cfg(test)]
mod global_pair_flags_runtime_tests;
#[cfg(test)]
mod evaluation_parameter_runtime_tests;
#[cfg(test)]
mod progression_reward_runtime_tests;
#[cfg(test)]
mod progression_gate_runtime_tests;
#[cfg(test)]
mod progression_selection_gate_runtime_tests;
#[cfg(test)]
mod trade_parameter_runtime_tests;
#[cfg(test)]
mod item_effect_parameter_runtime_tests;
#[cfg(test)]
mod audio_directory_runtime_tests;
#[cfg(test)]
mod audio_tone_runtime_tests;
#[cfg(test)]
mod audio_sequence_runtime_tests;
#[cfg(test)]
mod audio_driver_runtime_tests;
#[cfg(test)]
mod palette_effect_dispatch_runtime_tests;
#[cfg(test)]
mod palette_program_runtime_tests;
#[cfg(test)]
mod blend_program_runtime_tests;
#[cfg(test)]
mod task_descriptor_runtime_tests;
#[cfg(test)]
mod effect_event_directory_runtime_tests;
#[cfg(test)]
mod script_command_runtime_tests;
#[cfg(test)]
mod script_archive_runtime_tests;
#[cfg(test)]
mod script_text_runtime_tests;
#[cfg(test)]
mod script_coordinate_runtime_tests;
#[cfg(test)]
mod auxiliary_scripts_runtime_tests;
#[cfg(test)]
mod ui_choice_list_runtime_tests;
#[cfg(test)]
mod unit_obj_initial_pose_runtime_tests;
#[cfg(test)]
mod color_mode_palette_runtime_tests;
#[cfg(test)]
mod unit_obj_equipment_runtime_tests;
#[cfg(test)]
mod unit_obj_normal_battle_runtime_tests;
#[cfg(test)]
mod unit_obj_alternate_battle_runtime_tests;
#[cfg(test)]
mod script_special_normal_runtime_tests;
#[cfg(test)]
mod script_special_alternate_runtime_tests;
#[cfg(test)]
mod script_override_runtime_tests;
#[cfg(test)]
mod script_registry_special_runtime_tests;
#[cfg(test)]
mod script_registry_generic_runtime_tests;
#[cfg(test)]
mod unit_obj_property_words_runtime_tests;
#[cfg(test)]
mod unit_obj_byte_property_words_runtime_tests;
#[cfg(test)]
mod unit_obj_ffff_reader_fields_runtime_tests;
#[cfg(test)]
mod slot_job_lookup_runtime_tests;
#[cfg(test)]
mod observed_task_parameters_runtime_tests;
#[cfg(test)]
mod phase_zero_threshold_tests;
#[cfg(test)]
mod map_raster_palette_runtime_tests;
#[cfg(test)]
mod aggregate_flag_key_runtime_tests;
#[cfg(test)]
mod metadata52_runtime_tests;
#[cfg(test)]
mod metadata16_runtime_tests;
#[cfg(test)]
mod item_link_runtime_tests;
#[cfg(test)]
mod item_runtime_tests;
#[cfg(test)]
mod ui_localized_graphics_tests;
#[cfg(test)]
mod ui_localized_graphics_runtime_tests;
#[cfg(test)]
mod table_graphics_runtime_tests;
#[cfg(test)]
mod ui_graphics_runtime_tests;
#[cfg(test)]
mod ui_palette_runtime_tests;
#[cfg(test)]
mod obj_graphics_runtime_tests;
#[cfg(test)]
mod obj_animation_runtime_tests;
#[cfg(test)]
mod font_keyboard_runtime_tests;
#[cfg(test)]
mod bg_packet_tests;
#[cfg(test)]
mod bg_packet_catalog_tests;
#[cfg(test)]
mod bg_packet_data_tests;
#[cfg(test)]
mod bg_packet_managed_tests;
#[cfg(test)]
mod bg_packet_runtime_tests;
#[cfg(test)]
mod bg_palette_tests;
#[cfg(test)]
mod bg_runtime_tests;
#[cfg(test)]
mod compression_tests;
#[cfg(test)]
mod managed_tests;
#[cfg(test)]
mod native_tests;
#[cfg(test)]
mod palette_data_tests;
#[cfg(test)]
mod palette_runtime_tests;
mod preparation;
pub(crate) mod diagnosis;
#[cfg(test)]
mod validation_tests;
#[cfg(test)]
mod native_rom_data_residue_tests;
#[cfg(all(test, feature = "rom-data-read-audit"))]
mod host_rom_read_audit_tests;
#[cfg(all(test, feature = "rom-data-read-audit"))]
mod lower_rom_read_sites_tests;
#[cfg(all(test, feature = "rom-data-read-audit"))]
mod bank_rom_read_audit_tests;
#[cfg(all(test, feature = "rom-data-read-audit"))]
mod world_location_read_audit_tests;
pub use preparation::prepare_startup;

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| format!("Cannot inspect {}: {e}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.len() > limit {
        return Err(format!(
            "Resource must be a regular file of at most {limit} bytes: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|f| f.take(limit + 1).read_to_end(&mut bytes))
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    if bytes.len() as u64 > limit {
        return Err("Resource grew beyond the file size limit".into());
    }
    Ok(bytes)
}

fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T, String> {
    let value = std::str::from_utf8(bytes).map_err(|_| format!("{label} is not UTF-8"))?;
    toml::from_str(value).map_err(|e| format!("Invalid {label}: {e}"))
}

fn parse_background(bytes: &[u8]) -> Result<Descriptors, String> {
    // Deserialize directly into bounded typed records. A generic TOML/JSON
    // document tree needlessly multiplies temporary memory for this family.
    let result: Descriptors = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid background descriptors: {e}"))?;
    if result.entries.is_empty()
        || result.entries.len() > gba_core::rom_data::bg_descriptors::MAX_RECORDS
    {
        return Err("Invalid background descriptor count".into());
    }
    Ok(result)
}

fn encode_background(background: &Descriptors) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(background)
        .map_err(|e| format!("Cannot encode background descriptors: {e}"))
}

fn parse_palettes(bytes: &[u8]) -> Result<Palettes, String> {
    let palettes: Palettes =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid base palettes: {e}"))?;
    palettes.validate()?;
    Ok(palettes)
}

fn encode_palettes(palettes: &Palettes) -> Result<Vec<u8>, String> {
    palettes.validate()?;
    serde_json::to_vec_pretty(palettes).map_err(|e| format!("Cannot encode base palettes: {e}"))
}

fn parse_alternate_palettes(bytes: &[u8]) -> Result<AlternatePalettes, String> {
    let palettes: AlternatePalettes = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid alternate palettes: {e}"))?;
    palettes.validate()?;
    Ok(palettes)
}

fn encode_alternate_palettes(palettes: &AlternatePalettes) -> Result<Vec<u8>, String> {
    palettes.validate()?;
    serde_json::to_vec_pretty(palettes).map_err(|e| format!("Cannot encode alternate palettes: {e}"))
}

fn parse_packet_programs(bytes: &[u8]) -> Result<PacketPrograms, String> {
    let programs: PacketPrograms = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid packet programs: {e}"))?;
    programs.validate()?;
    Ok(programs)
}

fn encode_packet_programs(programs: &PacketPrograms) -> Result<Vec<u8>, String> {
    programs.validate()?;
    serde_json::to_vec_pretty(programs).map_err(|e| format!("Cannot encode packet programs: {e}"))
}

fn parse_main_graphics(bytes: &[u8]) -> Result<Graphics, String> {
    let graphics: Graphics = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid main graphics: {e}"))?;
    graphics.validate()?;
    Ok(graphics)
}

fn encode_main_graphics(graphics: &Graphics) -> Result<Vec<u8>, String> {
    graphics.validate()?;
    serde_json::to_vec_pretty(graphics).map_err(|e| format!("Cannot encode main graphics: {e}"))
}

fn parse_transfers(bytes: &[u8]) -> Result<Transfers, String> {
    let transfers: Transfers = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid transfer programs: {e}"))?;
    transfers.validate()?;
    Ok(transfers)
}

fn encode_transfers(transfers: &Transfers) -> Result<Vec<u8>, String> {
    transfers.validate()?;
    serde_json::to_vec_pretty(transfers).map_err(|e| format!("Cannot encode transfer programs: {e}"))
}

fn parse_short_texts(bytes: &[u8]) -> Result<ShortTexts, String> {
    let value: ShortTexts = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid short texts: {e}"))?;
    value.validate()?;
    Ok(value)
}

fn encode_short_texts(value: &ShortTexts) -> Result<Vec<u8>, String> {
    value.validate()?;
    serde_json::to_vec_pretty(value).map_err(|e| format!("Cannot encode short texts: {e}"))
}

fn parse_metadata16(bytes:&[u8])->Result<Metadata16,String> {
    let value:Metadata16=serde_json::from_slice(bytes).map_err(|e|format!("Invalid metadata16: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_metadata16(value:&Metadata16)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode metadata16: {e}"))}

fn parse_metadata52(bytes:&[u8])->Result<Metadata52,String> {
    let value:Metadata52=serde_json::from_slice(bytes).map_err(|e|format!("Invalid metadata52: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_metadata52(value:&Metadata52)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode metadata52: {e}"))}

fn parse_metadata4(bytes:&[u8])->Result<Metadata4,String> {
    let value:Metadata4=serde_json::from_slice(bytes).map_err(|e|format!("Invalid metadata4: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_metadata4(value:&Metadata4)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode metadata4: {e}"))}
fn parse_metadata2(bytes:&[u8])->Result<Metadata2,String> {
    let value:Metadata2=serde_json::from_slice(bytes).map_err(|e|format!("Invalid metadata2: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_metadata2(value:&Metadata2)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode metadata2: {e}"))}
fn parse_default48(bytes:&[u8])->Result<Default48,String> {
    let value:Default48=serde_json::from_slice(bytes).map_err(|e|format!("Invalid default48: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_default48(value:&Default48)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode default48: {e}"))}
fn parse_constructor_alternates(bytes:&[u8])->Result<ConstructorAlternates,String> {
    let value:ConstructorAlternates=serde_json::from_slice(bytes).map_err(|e|format!("Invalid constructor_alternates: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_constructor_alternates(value:&ConstructorAlternates)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode constructor_alternates: {e}"))}
fn parse_metadata14(bytes:&[u8])->Result<Metadata14,String> {
    let value:Metadata14=serde_json::from_slice(bytes).map_err(|e|format!("Invalid metadata14: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_metadata14(value:&Metadata14)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode metadata14: {e}"))}
fn parse_metadata12(bytes:&[u8])->Result<Metadata12,String> {
    let value:Metadata12=serde_json::from_slice(bytes).map_err(|e|format!("Invalid metadata12: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_metadata12(value:&Metadata12)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode metadata12: {e}"))}
fn parse_global_pair_flags(bytes:&[u8])->Result<GlobalPairFlags,String> {
    let value:GlobalPairFlags=serde_json::from_slice(bytes).map_err(|e|format!("Invalid global_pair_flags: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_global_pair_flags(value:&GlobalPairFlags)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode global_pair_flags: {e}"))}
fn parse_evaluation_parameters(bytes:&[u8])->Result<EvaluationParameters,String> {
    let value:EvaluationParameters=serde_json::from_slice(bytes).map_err(|e|format!("Invalid evaluation_parameters: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_evaluation_parameters(value:&EvaluationParameters)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode evaluation_parameters: {e}"))}
fn parse_progression_rewards(bytes:&[u8])->Result<ProgressionRewards,String> {
    let value:ProgressionRewards=serde_json::from_slice(bytes).map_err(|e|format!("Invalid progression_rewards: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_progression_rewards(value:&ProgressionRewards)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode progression_rewards: {e}"))}
fn parse_progression_gates(bytes:&[u8])->Result<ProgressionGates,String> {
    let value:ProgressionGates=serde_json::from_slice(bytes).map_err(|e|format!("Invalid progression_gates: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_progression_gates(value:&ProgressionGates)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode progression_gates: {e}"))}
fn parse_progression_selection_gates(bytes:&[u8])->Result<ProgressionSelectionGates,String> {
    let value:ProgressionSelectionGates=serde_json::from_slice(bytes).map_err(|e|format!("Invalid progression_selection_gates: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_progression_selection_gates(value:&ProgressionSelectionGates)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode progression_selection_gates: {e}"))}
fn parse_aggregate_flag_keys(bytes:&[u8])->Result<AggregateFlagKeys,String> {
    let value:AggregateFlagKeys=serde_json::from_slice(bytes).map_err(|e|format!("Invalid aggregate_flag_keys: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_aggregate_flag_keys(value:&AggregateFlagKeys)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode aggregate_flag_keys: {e}"))}
fn parse_trade_parameters(bytes:&[u8])->Result<TradeParameters,String> {
    let value:TradeParameters=serde_json::from_slice(bytes).map_err(|e|format!("Invalid trade_parameters: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_trade_parameters(value:&TradeParameters)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode trade_parameters: {e}"))}
fn parse_item_effect_parameters(bytes:&[u8])->Result<ItemEffectParameters,String> {
    let value:ItemEffectParameters=serde_json::from_slice(bytes).map_err(|e|format!("Invalid item_effect_parameters: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_item_effect_parameters(value:&ItemEffectParameters)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode item_effect_parameters: {e}"))}
fn parse_audio_directory(bytes:&[u8])->Result<AudioDirectory,String> {
    let value:AudioDirectory=serde_json::from_slice(bytes).map_err(|e|format!("Invalid audio_directory: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_audio_directory(value:&AudioDirectory)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode audio_directory: {e}"))}
fn parse_audio_tones(bytes:&[u8])->Result<AudioTones,String> {
    let value:AudioTones=serde_json::from_slice(bytes).map_err(|e|format!("Invalid audio_tones: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_audio_tones(value:&AudioTones)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode audio_tones: {e}"))}
fn parse_audio_sequences(bytes:&[u8])->Result<AudioSequences,String> {
    let value:AudioSequences=serde_json::from_slice(bytes).map_err(|e|format!("Invalid audio_sequences: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_audio_sequences(value:&AudioSequences)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode audio_sequences: {e}"))}
fn parse_audio_driver(bytes:&[u8])->Result<AudioDriver,String> {
    let value:AudioDriver=serde_json::from_slice(bytes).map_err(|e|format!("Invalid audio_driver: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_audio_driver(value:&AudioDriver)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode audio_driver: {e}"))}
fn parse_palette_effect_dispatch(bytes:&[u8])->Result<PaletteEffectDispatch,String> {
    let value:PaletteEffectDispatch=serde_json::from_slice(bytes).map_err(|e|format!("Invalid palette_effect_dispatch: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_palette_effect_dispatch(value:&PaletteEffectDispatch)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode palette_effect_dispatch: {e}"))}
fn parse_palette_programs(bytes:&[u8])->Result<PalettePrograms,String> {
    let value:PalettePrograms=serde_json::from_slice(bytes).map_err(|e|format!("Invalid palette_programs: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_palette_programs(value:&PalettePrograms)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode palette_programs: {e}"))}
fn parse_blend_programs(bytes:&[u8])->Result<BlendPrograms,String> {
    let value:BlendPrograms=serde_json::from_slice(bytes).map_err(|e|format!("Invalid blend_programs: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_blend_programs(value:&BlendPrograms)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode blend_programs: {e}"))}
fn parse_task_descriptors(bytes:&[u8])->Result<TaskDescriptors,String> {
    let value:TaskDescriptors=serde_json::from_slice(bytes).map_err(|e|format!("Invalid task_descriptors: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_task_descriptors(value:&TaskDescriptors)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode task_descriptors: {e}"))}
fn parse_effect_event_directories(bytes:&[u8])->Result<EffectEventDirectories,String> {
    let value:EffectEventDirectories=serde_json::from_slice(bytes).map_err(|e|format!("Invalid effect_event_directories: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_effect_event_directories(value:&EffectEventDirectories)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode effect_event_directories: {e}"))}
fn parse_script_commands(bytes:&[u8])->Result<ScriptCommands,String> {
    let value:ScriptCommands=serde_json::from_slice(bytes).map_err(|e|format!("Invalid script_commands: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_script_commands(value:&ScriptCommands)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode script_commands: {e}"))}
fn parse_script_archive(bytes:&[u8])->Result<ScriptArchive,String> {
    let value:ScriptArchive=serde_json::from_slice(bytes).map_err(|e|format!("Invalid script_archive: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_script_archive(value:&ScriptArchive)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode script_archive: {e}"))}
fn parse_script_text(bytes:&[u8])->Result<ScriptText,String> {
    let value:ScriptText=serde_json::from_slice(bytes).map_err(|e|format!("Invalid script_text: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_script_text(value:&ScriptText)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode script_text: {e}"))}
fn parse_script_coordinates(bytes:&[u8])->Result<ScriptCoordinates,String> {
    let value:ScriptCoordinates=serde_json::from_slice(bytes).map_err(|e|format!("Invalid script_coordinates: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_script_coordinates(value:&ScriptCoordinates)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode script_coordinates: {e}"))}
fn parse_ui_choice_lists(bytes:&[u8])->Result<UiChoiceLists,String> {
    let value:UiChoiceLists=serde_json::from_slice(bytes).map_err(|e|format!("Invalid ui_choice_lists: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_ui_choice_lists(value:&UiChoiceLists)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode ui_choice_lists: {e}"))}
fn parse_unit_obj_initial_pose(bytes:&[u8])->Result<UnitObjInitialPose,String> {
    let value:UnitObjInitialPose=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_initial_pose: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_initial_pose(value:&UnitObjInitialPose)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_initial_pose: {e}"))}
fn parse_color_mode_palettes(bytes:&[u8])->Result<ColorModePalettes,String> {
    let value:ColorModePalettes=serde_json::from_slice(bytes).map_err(|e|format!("Invalid color_mode_palettes: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_color_mode_palettes(value:&ColorModePalettes)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode color_mode_palettes: {e}"))}
fn parse_unit_obj_equipment(bytes:&[u8])->Result<UnitObjEquipment,String> {
    let value:UnitObjEquipment=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_equipment: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_equipment(value:&UnitObjEquipment)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_equipment: {e}"))}
fn parse_unit_obj_normal_battle(bytes:&[u8])->Result<UnitObjNormalBattle,String> {
    let value:UnitObjNormalBattle=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_normal_battle: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_normal_battle(value:&UnitObjNormalBattle)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_normal_battle: {e}"))}
fn parse_unit_obj_alternate_battle(bytes:&[u8])->Result<UnitObjAlternateBattle,String> {
    let value:UnitObjAlternateBattle=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_alternate_battle: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_alternate_battle(value:&UnitObjAlternateBattle)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_alternate_battle: {e}"))}
fn parse_unit_obj_script_special_normal(bytes:&[u8])->Result<UnitObjScriptSpecialNormal,String> {
    let value:UnitObjScriptSpecialNormal=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_script_special_normal: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_script_special_normal(value:&UnitObjScriptSpecialNormal)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_script_special_normal: {e}"))}
fn parse_unit_obj_script_special_alternate(bytes:&[u8])->Result<UnitObjScriptSpecialAlternate,String> {
    let value:UnitObjScriptSpecialAlternate=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_script_special_alternate: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_script_special_alternate(value:&UnitObjScriptSpecialAlternate)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_script_special_alternate: {e}"))}
fn parse_unit_obj_script_override(bytes:&[u8])->Result<UnitObjScriptOverride,String> {
    let value:UnitObjScriptOverride=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_script_override: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_script_override(value:&UnitObjScriptOverride)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_script_override: {e}"))}
fn parse_unit_obj_script_registry_special(bytes:&[u8])->Result<UnitObjScriptRegistrySpecial,String> {
    let value:UnitObjScriptRegistrySpecial=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_script_registry_special: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_script_registry_special(value:&UnitObjScriptRegistrySpecial)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_script_registry_special: {e}"))}
fn parse_unit_obj_script_registry_generic(bytes:&[u8])->Result<UnitObjScriptRegistryGeneric,String> {
    let value:UnitObjScriptRegistryGeneric=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_script_registry_generic: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_script_registry_generic(value:&UnitObjScriptRegistryGeneric)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_script_registry_generic: {e}"))}
fn parse_unit_obj_property_words(bytes:&[u8])->Result<UnitObjPropertyWords,String> {
    let value:UnitObjPropertyWords=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_property_words: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_property_words(value:&UnitObjPropertyWords)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_property_words: {e}"))}
fn parse_unit_obj_byte_property_words(bytes:&[u8])->Result<UnitObjBytePropertyWords,String> {
    let value:UnitObjBytePropertyWords=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_byte_property_words: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_byte_property_words(value:&UnitObjBytePropertyWords)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_byte_property_words: {e}"))}
fn parse_unit_obj_ffff_reader_fields(bytes:&[u8])->Result<UnitObjFfffReaderFields,String> {
    let value:UnitObjFfffReaderFields=serde_json::from_slice(bytes).map_err(|e|format!("Invalid unit_obj_ffff_reader_fields: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_unit_obj_ffff_reader_fields(value:&UnitObjFfffReaderFields)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode unit_obj_ffff_reader_fields: {e}"))}
fn parse_slot_job_lookup(bytes:&[u8])->Result<SlotJobLookup,String> {
    let value:SlotJobLookup=serde_json::from_slice(bytes).map_err(|e|format!("Invalid slot_job_lookup: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_slot_job_lookup(value:&SlotJobLookup)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode slot_job_lookup: {e}"))}
fn parse_observed_task_parameters(bytes:&[u8])->Result<ObservedTaskParameters,String> {
    let value:ObservedTaskParameters=serde_json::from_slice(bytes).map_err(|e|format!("Invalid observed_task_parameters: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_observed_task_parameters(value:&ObservedTaskParameters)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode observed_task_parameters: {e}"))}
fn parse_constructor_obj_animation(bytes:&[u8])->Result<ConstructorObjAnimation,String> {
    let value:ConstructorObjAnimation=serde_json::from_slice(bytes).map_err(|e|format!("Invalid constructor_obj_animation: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_constructor_obj_animation(value:&ConstructorObjAnimation)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode constructor_obj_animation: {e}"))}
fn parse_task_cycle_deltas(bytes:&[u8])->Result<TaskCycleDeltas,String> {
    let value:TaskCycleDeltas=serde_json::from_slice(bytes).map_err(|e|format!("Invalid task_cycle_deltas: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_task_cycle_deltas(value:&TaskCycleDeltas)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode task_cycle_deltas: {e}"))}
fn parse_ui_raw_transfers(bytes:&[u8])->Result<RawTransfers,String> {let value:RawTransfers=serde_json::from_slice(bytes).map_err(|e|format!("Invalid ui_raw_transfers: {e}"))?;value.validate()?;Ok(value)}
fn parse_dynamic_raw_fragments(bytes:&[u8])->Result<DynamicRawFragments,String> {let value:DynamicRawFragments=serde_json::from_slice(bytes).map_err(|e|format!("Invalid dynamic_raw_fragments: {e}"))?;value.validate()?;Ok(value)}
fn parse_conditional_four_byte_lookup(bytes:&[u8])->Result<ConditionalFourByteLookup,String> {let value:ConditionalFourByteLookup=serde_json::from_slice(bytes).map_err(|e|format!("Invalid conditional_four_byte_lookup: {e}"))?;value.validate()?;Ok(value)}
fn encode_conditional_four_byte_lookup(value:&ConditionalFourByteLookup)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode conditional_four_byte_lookup: {e}"))}
fn parse_conditional_packed_lookup(bytes:&[u8])->Result<ConditionalPackedLookup,String> {let value:ConditionalPackedLookup=serde_json::from_slice(bytes).map_err(|e|format!("Invalid conditional_packed_lookup: {e}"))?;value.validate()?;Ok(value)}
fn encode_conditional_packed_lookup(value:&ConditionalPackedLookup)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode conditional_packed_lookup: {e}"))}
fn parse_conditional_descriptor_banks(bytes:&[u8])->Result<ConditionalDescriptorBanks,String> {let value:ConditionalDescriptorBanks=serde_json::from_slice(bytes).map_err(|e|format!("Invalid conditional_descriptor_banks: {e}"))?;value.validate()?;Ok(value)}
fn encode_conditional_descriptor_banks(value:&ConditionalDescriptorBanks)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode conditional_descriptor_banks: {e}"))}
fn parse_conditional_descriptor_bank_zero(bytes:&[u8])->Result<ConditionalDescriptorBankZero,String> {let value:ConditionalDescriptorBankZero=serde_json::from_slice(bytes).map_err(|e|format!("Invalid conditional_descriptor_bank_zero: {e}"))?;value.validate()?;Ok(value)}
fn encode_conditional_descriptor_bank_zero(value:&ConditionalDescriptorBankZero)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode conditional_descriptor_bank_zero: {e}"))}
fn parse_conditional_property_masks(bytes:&[u8])->Result<ConditionalPropertyMasks,String> {let value:ConditionalPropertyMasks=serde_json::from_slice(bytes).map_err(|e|format!("Invalid conditional_property_masks: {e}"))?;value.validate()?;Ok(value)}
fn parse_compact_obj_animations(bytes:&[u8])->Result<CompactObjAnimations,String> {let value:CompactObjAnimations=serde_json::from_slice(bytes).map_err(|e|format!("Invalid compact_obj_animations: {e}"))?;value.validate()?;Ok(value)}
fn parse_secondary_compact_obj_animations(bytes:&[u8])->Result<SecondaryCompactObjAnimations,String> {let value:SecondaryCompactObjAnimations=serde_json::from_slice(bytes).map_err(|e|format!("Invalid secondary_compact_obj_animations: {e}"))?;value.validate()?;Ok(value)}
fn parse_tertiary_compact_obj_animations(bytes:&[u8])->Result<TertiaryCompactObjAnimations,String> {let value:TertiaryCompactObjAnimations=serde_json::from_slice(bytes).map_err(|e|format!("Invalid tertiary_compact_obj_animations: {e}"))?;value.validate()?;Ok(value)}
fn parse_quaternary_compact_obj_animations(bytes:&[u8])->Result<QuaternaryCompactObjAnimations,String> {let value:QuaternaryCompactObjAnimations=serde_json::from_slice(bytes).map_err(|e|format!("Invalid quaternary_compact_obj_animations: {e}"))?;value.validate()?;Ok(value)}
fn parse_quinary_compact_obj_animations(bytes:&[u8])->Result<QuinaryCompactObjAnimations,String> {let value:QuinaryCompactObjAnimations=serde_json::from_slice(bytes).map_err(|e|format!("Invalid quinary_compact_obj_animations: {e}"))?;value.validate()?;Ok(value)}
fn parse_senary_compact_obj_animations(bytes:&[u8])->Result<SenaryCompactObjAnimations,String> {let value:SenaryCompactObjAnimations=serde_json::from_slice(bytes).map_err(|e|format!("Invalid senary_compact_obj_animations: {e}"))?;value.validate()?;Ok(value)}
fn encode_conditional_property_masks(value:&ConditionalPropertyMasks)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode conditional_property_masks: {e}"))}
fn encode_compact_obj_animations(value:&CompactObjAnimations)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode compact_obj_animations: {e}"))}
fn encode_secondary_compact_obj_animations(value:&SecondaryCompactObjAnimations)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode secondary_compact_obj_animations: {e}"))}
fn encode_tertiary_compact_obj_animations(value:&TertiaryCompactObjAnimations)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode tertiary_compact_obj_animations: {e}"))}
fn encode_quaternary_compact_obj_animations(value:&QuaternaryCompactObjAnimations)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode quaternary_compact_obj_animations: {e}"))}
fn encode_quinary_compact_obj_animations(value:&QuinaryCompactObjAnimations)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode quinary_compact_obj_animations: {e}"))}
fn encode_senary_compact_obj_animations(value:&SenaryCompactObjAnimations)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode senary_compact_obj_animations: {e}"))}
fn encode_ui_raw_transfers(value:&RawTransfers)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode ui_raw_transfers: {e}"))}
fn encode_dynamic_raw_fragments(value:&DynamicRawFragments)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode dynamic_raw_fragments: {e}"))}
fn parse_indexed_graphics(bytes:&[u8])->Result<IndexedGraphics,String> {let value:IndexedGraphics=serde_json::from_slice(bytes).map_err(|e|format!("Invalid indexed_graphics: {e}"))?;value.validate()?;Ok(value)}
fn encode_indexed_graphics(value:&IndexedGraphics)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode indexed_graphics: {e}"))}
fn parse_world_records38(bytes:&[u8])->Result<WorldRecords38,String> {let value:WorldRecords38=serde_json::from_slice(bytes).map_err(|e|format!("Invalid world_records38: {e}"))?;value.validate()?;Ok(value)}
fn encode_world_records38(value:&WorldRecords38)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode world_records38: {e}"))}
fn parse_parameter_records27(bytes:&[u8])->Result<ParameterRecords27,String> {let value:ParameterRecords27=serde_json::from_slice(bytes).map_err(|e|format!("Invalid parameter_records27: {e}"))?;value.validate()?;Ok(value)}
fn encode_parameter_records27(value:&ParameterRecords27)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode parameter_records27: {e}"))}
fn parse_event_records12(bytes:&[u8])->Result<EventRecords12,String> {let value:EventRecords12=serde_json::from_slice(bytes).map_err(|e|format!("Invalid event_records12: {e}"))?;value.validate()?;Ok(value)}
fn encode_event_records12(value:&EventRecords12)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode event_records12: {e}"))}
fn parse_selection_byte_tables(bytes:&[u8])->Result<ByteTables,String> {let value:ByteTables=serde_json::from_slice(bytes).map_err(|e|format!("Invalid selection_byte_tables: {e}"))?;value.validate()?;Ok(value)}
fn encode_selection_byte_tables(value:&ByteTables)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode selection_byte_tables: {e}"))}
fn parse_mission_records70(bytes:&[u8])->Result<Records70,String> {let value:Records70=serde_json::from_slice(bytes).map_err(|e|format!("Invalid mission_records70: {e}"))?;value.validate()?;Ok(value)}
fn encode_mission_records70(value:&Records70)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode mission_records70: {e}"))}
fn parse_conditional_records8(bytes:&[u8])->Result<Records8,String> {let value:Records8=serde_json::from_slice(bytes).map_err(|e|format!("Invalid conditional_records8: {e}"))?;value.validate()?;Ok(value)}
fn encode_conditional_records8(value:&Records8)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode conditional_records8: {e}"))}
fn parse_records4(bytes:&[u8])->Result<Records4,String> {let value:Records4=serde_json::from_slice(bytes).map_err(|e|format!("Invalid records4: {e}"))?;value.validate()?;Ok(value)}
fn encode_records4(value:&Records4)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode records4: {e}"))}
fn parse_metadata28(bytes:&[u8])->Result<Metadata28,String> {let value:Metadata28=serde_json::from_slice(bytes).map_err(|e|format!("Invalid metadata28: {e}"))?;value.validate()?;Ok(value)}
fn encode_metadata28(value:&Metadata28)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode metadata28: {e}"))}
fn parse_encounter_coordinates(bytes:&[u8])->Result<EncounterCoordinates,String> {let value:EncounterCoordinates=serde_json::from_slice(bytes).map_err(|e|format!("Invalid coordinates: {e}"))?;value.validate()?;Ok(value)}
fn encode_encounter_coordinates(value:&EncounterCoordinates)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode coordinates: {e}"))}
fn parse_encounter_unit_templates(bytes:&[u8])->Result<EncounterUnitTemplates,String> {
    let value:EncounterUnitTemplates=serde_json::from_slice(bytes).map_err(|e|format!("Invalid encounter_unit_templates: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_encounter_unit_templates(value:&EncounterUnitTemplates)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode encounter_unit_templates: {e}"))}
fn parse_encounter_headers(bytes:&[u8])->Result<EncounterHeaders,String> {
    let value:EncounterHeaders=serde_json::from_slice(bytes).map_err(|e|format!("Invalid encounter_headers: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_encounter_headers(value:&EncounterHeaders)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode encounter_headers: {e}"))}
fn parse_world_roads(bytes:&[u8])->Result<WorldRoads,String> {
    let value:WorldRoads=serde_json::from_slice(bytes).map_err(|e|format!("Invalid world_roads: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_world_roads(value:&WorldRoads)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode world_roads: {e}"))}
fn parse_world_nodes(bytes:&[u8])->Result<WorldNodes,String> {
    let value:WorldNodes=serde_json::from_slice(bytes).map_err(|e|format!("Invalid world_nodes: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_world_nodes(value:&WorldNodes)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode world_nodes: {e}"))}
fn parse_word_task_obj_animations(bytes:&[u8])->Result<WordTaskObjAnimations,String> {
    let value:WordTaskObjAnimations=serde_json::from_slice(bytes).map_err(|e|format!("Invalid word_task_obj_animations: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_word_task_obj_animations(value:&WordTaskObjAnimations)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode word_task_obj_animations: {e}"))}
fn parse_map_raster_palettes(bytes:&[u8])->Result<MapRasterPalettes,String> {
    let value:MapRasterPalettes=serde_json::from_slice(bytes).map_err(|e|format!("Invalid map_raster_palettes: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_map_raster_palettes(value:&MapRasterPalettes)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode map_raster_palettes: {e}"))}
fn parse_auxiliary_commands(bytes:&[u8])->Result<AuxiliaryCommands,String> {
    let value:AuxiliaryCommands=serde_json::from_slice(bytes).map_err(|e|format!("Invalid auxiliary_commands: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_auxiliary_commands(value:&AuxiliaryCommands)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode auxiliary_commands: {e}"))}
fn parse_auxiliary_archive(bytes:&[u8])->Result<AuxiliaryArchive,String> {
    let value:AuxiliaryArchive=serde_json::from_slice(bytes).map_err(|e|format!("Invalid auxiliary_archive: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_auxiliary_archive(value:&AuxiliaryArchive)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode auxiliary_archive: {e}"))}

fn parse_item_links(bytes:&[u8])->Result<ItemLinks,String> {
    let value:ItemLinks=serde_json::from_slice(bytes).map_err(|e|format!("Invalid item links: {e}"))?;
    value.validate()?;Ok(value)
}
fn encode_item_links(value:&ItemLinks)->Result<Vec<u8>,String> {serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode item links: {e}"))}

fn parse_items(bytes: &[u8]) -> Result<Items, String> {
    let value: Items = serde_json::from_slice(bytes).map_err(|e|format!("Invalid item records: {e}"))?;
    value.validate()?;
    Ok(value)
}
fn encode_items(value: &Items) -> Result<Vec<u8>,String> {
    serde_json::to_vec_pretty(value).map_err(|e|format!("Cannot encode item records: {e}"))
}

fn parse_ui_graphics(bytes: &[u8]) -> Result<UiGraphics, String> {
    let value: UiGraphics = serde_json::from_slice(bytes).map_err(|e| format!("Invalid UI custom graphics: {e}"))?;
    value.validate()?;
    Ok(value)
}
fn encode_ui_graphics(value: &UiGraphics) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(value).map_err(|e| format!("Cannot encode UI custom graphics: {e}"))
}

fn parse_ui_palettes(bytes: &[u8]) -> Result<UiPalettes, String> {
    let value: UiPalettes = serde_json::from_slice(bytes).map_err(|e| format!("Invalid UI raw palettes: {e}"))?;
    value.validate()?;
    Ok(value)
}
fn encode_ui_palettes(value: &UiPalettes) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(value).map_err(|e| format!("Cannot encode UI raw palettes: {e}"))
}

fn parse_obj_graphics(bytes: &[u8]) -> Result<ObjGraphics, String> {
    let value: ObjGraphics = serde_json::from_slice(bytes).map_err(|e| format!("Invalid OBJ custom graphics: {e}"))?;
    value.validate()?;
    Ok(value)
}
fn encode_obj_graphics(value: &ObjGraphics) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(value).map_err(|e| format!("Cannot encode OBJ custom graphics: {e}"))
}

fn parse_obj_animations(bytes: &[u8]) -> Result<ObjAnimationBanks, String> {
    let value: ObjAnimationBanks = serde_json::from_slice(bytes).map_err(|e| format!("Invalid OBJ animation banks: {e}"))?;
    value.validate()?; Ok(value)
}
fn encode_obj_animations(value: &ObjAnimationBanks) -> Result<Vec<u8>, String> {
    value.validate()?; serde_json::to_vec_pretty(value).map_err(|e| format!("Cannot encode OBJ animation banks: {e}"))
}

fn parse_obj_lists(bytes: &[u8]) -> Result<ObjLists, String> {
    let value: ObjLists = serde_json::from_slice(bytes).map_err(|e| format!("Invalid OBJ static lists: {e}"))?;
    value.validate()?;
    Ok(value)
}
fn encode_obj_lists(value: &ObjLists) -> Result<Vec<u8>, String> {
    value.validate()?;
    serde_json::to_vec_pretty(value).map_err(|e| format!("Cannot encode OBJ static lists: {e}"))
}

fn parse_font_bank(bytes: &[u8]) -> Result<FontBank, String> {
    let value: FontBank = serde_json::from_slice(bytes).map_err(|e| format!("Invalid font-bank: {e}"))?;
    value.validate()?;
    Ok(value)
}
fn encode_font_bank(value: &FontBank) -> Result<Vec<u8>, String> {
    value.validate()?;
    serde_json::to_vec_pretty(value).map_err(|e| format!("Cannot encode font-bank: {e}"))
}

fn parse_name_keyboard(bytes: &[u8]) -> Result<Keyboard, String> {
    let value: Keyboard = serde_json::from_slice(bytes).map_err(|e| format!("Invalid name-keyboard: {e}"))?;
    value.validate()?;
    Ok(value)
}
fn encode_name_keyboard(value: &Keyboard) -> Result<Vec<u8>, String> {
    value.validate()?;
    serde_json::to_vec_pretty(value).map_err(|e| format!("Cannot encode name-keyboard: {e}"))
}

fn parse_text_catalogs(bytes: &[u8]) -> Result<TextCatalogs, String> {
    let value: TextCatalogs = serde_json::from_slice(bytes).map_err(|e| format!("Invalid text catalogs: {e}"))?;
    value.validate()?; Ok(value)
}
fn encode_text_catalogs(value: &TextCatalogs) -> Result<Vec<u8>, String> {
    value.validate()?; serde_json::to_vec_pretty(value).map_err(|e| format!("Cannot encode text catalogs: {e}"))
}

pub fn verify(path: &Path, source: &Extraction) -> Result<ObjDimensions, String> {
    let dataset = read_dataset(path)?;
    source.validate_files(dataset.files())?;
    Ok(dataset.dimensions)
}

struct Dataset {
    manifest: Manifest,
    dimensions: ObjDimensions,
    obj_bytes: Vec<u8>,
    background: Descriptors,
    bg_bytes: Vec<u8>,
    palettes: Palettes,
    palette_bytes: Vec<u8>,
    alternate_palettes: AlternatePalettes,
    alternate_palette_bytes: Vec<u8>,
    packet_programs: PacketPrograms,
    packet_program_bytes: Vec<u8>,
    main_graphics: Option<Graphics>,
    main_graphic_bytes: Option<Vec<u8>>,
    transfers: Option<Transfers>,
    transfer_bytes: Option<Vec<u8>>,
    short_texts: Option<ShortTexts>,
    short_text_bytes: Option<Vec<u8>>,
    font_bank: Option<FontBank>,
    font_bank_bytes: Option<Vec<u8>>,
    name_keyboard: Option<Keyboard>,
    name_keyboard_bytes: Option<Vec<u8>>,
    obj_lists: Option<ObjLists>,
    obj_list_bytes: Option<Vec<u8>>,
    obj_animations: Option<ObjAnimationBanks>,
    obj_animation_bytes: Option<Vec<u8>>,
    obj_graphics: Option<ObjGraphics>,
    obj_graphic_bytes: Option<Vec<u8>>,
    ui_palettes: Option<UiPalettes>,
    ui_palette_bytes: Option<Vec<u8>>,
    ui_graphics: Option<UiGraphics>,
    ui_graphic_bytes: Option<Vec<u8>>,
    items: Option<Items>,
    item_bytes: Option<Vec<u8>>,
    item_links: Option<ItemLinks>,
    metadata16: Option<Metadata16>,
    metadata52: Option<Metadata52>,
    metadata4: Option<Metadata4>,
    metadata2: Option<Metadata2>,
    default48: Option<Default48>,
    constructor_alternates: Option<ConstructorAlternates>,
    metadata14: Option<Metadata14>,
    metadata12: Option<Metadata12>,
    global_pair_flags: Option<GlobalPairFlags>,
    evaluation_parameters: Option<EvaluationParameters>,
    progression_rewards: Option<ProgressionRewards>,
    progression_gates: Option<ProgressionGates>,
    progression_selection_gates: Option<ProgressionSelectionGates>,
    aggregate_flag_keys: Option<AggregateFlagKeys>,
    trade_parameters: Option<TradeParameters>,
    item_effect_parameters: Option<ItemEffectParameters>,
    audio_directory: Option<AudioDirectory>,
    audio_tones: Option<AudioTones>,
    audio_sequences: Option<AudioSequences>,
    audio_driver: Option<AudioDriver>,
    palette_effect_dispatch: Option<PaletteEffectDispatch>,
    palette_programs: Option<PalettePrograms>,
    blend_programs: Option<BlendPrograms>,
    task_descriptors: Option<TaskDescriptors>,
    effect_event_directories: Option<EffectEventDirectories>,
    script_commands: Option<ScriptCommands>,
    script_archive: Option<ScriptArchive>,
    script_text: Option<ScriptText>,
    script_coordinates: Option<ScriptCoordinates>,
    ui_choice_lists: Option<UiChoiceLists>,
    unit_obj_initial_pose: Option<UnitObjInitialPose>,
    color_mode_palettes: Option<ColorModePalettes>,
    unit_obj_equipment: Option<UnitObjEquipment>,
    unit_obj_normal_battle: Option<UnitObjNormalBattle>,
    unit_obj_alternate_battle: Option<UnitObjAlternateBattle>,
    unit_obj_script_special_normal: Option<UnitObjScriptSpecialNormal>,
    unit_obj_script_special_alternate: Option<UnitObjScriptSpecialAlternate>,
    unit_obj_script_override: Option<UnitObjScriptOverride>,
    unit_obj_script_registry_special: Option<UnitObjScriptRegistrySpecial>,
    unit_obj_script_registry_generic: Option<UnitObjScriptRegistryGeneric>,
    unit_obj_property_words: Option<UnitObjPropertyWords>,
    unit_obj_byte_property_words: Option<UnitObjBytePropertyWords>,
    unit_obj_ffff_reader_fields: Option<UnitObjFfffReaderFields>,
    slot_job_lookup: Option<SlotJobLookup>,
    observed_task_parameters: Option<ObservedTaskParameters>,
    constructor_obj_animation: Option<ConstructorObjAnimation>,
    task_cycle_deltas: Option<TaskCycleDeltas>,
    word_task_obj_animations: Option<WordTaskObjAnimations>,
    world_nodes: Option<WorldNodes>,
    world_roads: Option<WorldRoads>,
    encounter_headers: Option<EncounterHeaders>,
    encounter_unit_templates: Option<EncounterUnitTemplates>,
    encounter_coordinates: Option<EncounterCoordinates>,
    metadata28: Option<Metadata28>,
    records4: Option<Records4>,
    conditional_records8: Option<Records8>,
    mission_records70: Option<Records70>,
    selection_byte_tables: Option<ByteTables>,
    event_records12: Option<EventRecords12>,
    parameter_records27: Option<ParameterRecords27>,
    world_records38: Option<WorldRecords38>,
    indexed_graphics: Option<IndexedGraphics>,
    ui_raw_transfers: Option<RawTransfers>,
    dynamic_raw_fragments: Option<DynamicRawFragments>,
    conditional_four_byte_lookup: Option<ConditionalFourByteLookup>,
    conditional_packed_lookup: Option<ConditionalPackedLookup>,
    conditional_descriptor_banks: Option<ConditionalDescriptorBanks>,
    conditional_descriptor_bank_zero: Option<ConditionalDescriptorBankZero>,
    conditional_property_masks: Option<ConditionalPropertyMasks>,
    compact_obj_animations: Option<CompactObjAnimations>,
    secondary_compact_obj_animations: Option<SecondaryCompactObjAnimations>,
    tertiary_compact_obj_animations: Option<TertiaryCompactObjAnimations>,
    quaternary_compact_obj_animations: Option<QuaternaryCompactObjAnimations>,
    quinary_compact_obj_animations: Option<QuinaryCompactObjAnimations>,
    senary_compact_obj_animations: Option<SenaryCompactObjAnimations>,
    map_raster_palettes: Option<MapRasterPalettes>,
    auxiliary_commands: Option<AuxiliaryCommands>,
    auxiliary_archive: Option<AuxiliaryArchive>,
    metadata16_bytes: Option<Vec<u8>>,
    metadata52_bytes: Option<Vec<u8>>,
    metadata4_bytes: Option<Vec<u8>>,
    metadata2_bytes: Option<Vec<u8>>,
    default48_bytes: Option<Vec<u8>>,
    constructor_alternates_bytes: Option<Vec<u8>>,
    metadata14_bytes: Option<Vec<u8>>,
    metadata12_bytes: Option<Vec<u8>>,
    global_pair_flags_bytes: Option<Vec<u8>>,
    evaluation_parameters_bytes: Option<Vec<u8>>,
    progression_rewards_bytes: Option<Vec<u8>>,
    progression_gates_bytes: Option<Vec<u8>>,
    progression_selection_gates_bytes: Option<Vec<u8>>,
    aggregate_flag_keys_bytes: Option<Vec<u8>>,
    trade_parameters_bytes: Option<Vec<u8>>,
    item_effect_parameters_bytes: Option<Vec<u8>>,
    audio_directory_bytes: Option<Vec<u8>>,
    audio_tones_bytes: Option<Vec<u8>>,
    audio_sequences_bytes: Option<Vec<u8>>,
    audio_driver_bytes: Option<Vec<u8>>,
    palette_effect_dispatch_bytes: Option<Vec<u8>>,
    palette_programs_bytes: Option<Vec<u8>>,
    blend_programs_bytes: Option<Vec<u8>>,
    task_descriptors_bytes: Option<Vec<u8>>,
    effect_event_directories_bytes: Option<Vec<u8>>,
    script_commands_bytes: Option<Vec<u8>>,
    script_archive_bytes: Option<Vec<u8>>,
    script_text_bytes: Option<Vec<u8>>,
    script_coordinates_bytes: Option<Vec<u8>>,
    ui_choice_lists_bytes: Option<Vec<u8>>,
    unit_obj_initial_pose_bytes: Option<Vec<u8>>,
    color_mode_palettes_bytes: Option<Vec<u8>>,
    unit_obj_equipment_bytes: Option<Vec<u8>>,
    unit_obj_normal_battle_bytes: Option<Vec<u8>>,
    unit_obj_alternate_battle_bytes: Option<Vec<u8>>,
    unit_obj_script_special_normal_bytes: Option<Vec<u8>>,
    unit_obj_script_special_alternate_bytes: Option<Vec<u8>>,
    unit_obj_script_override_bytes: Option<Vec<u8>>,
    unit_obj_script_registry_special_bytes: Option<Vec<u8>>,
    unit_obj_script_registry_generic_bytes: Option<Vec<u8>>,
    unit_obj_property_words_bytes: Option<Vec<u8>>,
    unit_obj_byte_property_words_bytes: Option<Vec<u8>>,
    unit_obj_ffff_reader_fields_bytes: Option<Vec<u8>>,
    slot_job_lookup_bytes: Option<Vec<u8>>,
    observed_task_parameters_bytes: Option<Vec<u8>>,
    constructor_obj_animation_bytes: Option<Vec<u8>>,
    task_cycle_deltas_bytes: Option<Vec<u8>>,
    word_task_obj_animations_bytes: Option<Vec<u8>>,
    world_nodes_bytes: Option<Vec<u8>>,
    world_roads_bytes: Option<Vec<u8>>,
    encounter_headers_bytes: Option<Vec<u8>>,
    encounter_unit_templates_bytes: Option<Vec<u8>>,
    encounter_coordinates_bytes: Option<Vec<u8>>,
    metadata28_bytes: Option<Vec<u8>>,
    records4_bytes: Option<Vec<u8>>,
    conditional_records8_bytes: Option<Vec<u8>>,
    mission_records70_bytes: Option<Vec<u8>>,
    selection_byte_tables_bytes: Option<Vec<u8>>,
    event_records12_bytes: Option<Vec<u8>>,
    parameter_records27_bytes: Option<Vec<u8>>,
    world_records38_bytes: Option<Vec<u8>>,
    indexed_graphics_bytes: Option<Vec<u8>>,
    ui_raw_transfers_bytes: Option<Vec<u8>>,
    dynamic_raw_fragments_bytes: Option<Vec<u8>>,
    conditional_four_byte_lookup_bytes: Option<Vec<u8>>,
    conditional_packed_lookup_bytes: Option<Vec<u8>>,
    conditional_descriptor_banks_bytes: Option<Vec<u8>>,
    conditional_descriptor_bank_zero_bytes: Option<Vec<u8>>,
    conditional_property_masks_bytes: Option<Vec<u8>>,
    compact_obj_animations_bytes: Option<Vec<u8>>,
    secondary_compact_obj_animations_bytes: Option<Vec<u8>>,
    tertiary_compact_obj_animations_bytes: Option<Vec<u8>>,
    quaternary_compact_obj_animations_bytes: Option<Vec<u8>>,
    quinary_compact_obj_animations_bytes: Option<Vec<u8>>,
    senary_compact_obj_animations_bytes: Option<Vec<u8>>,
    map_raster_palettes_bytes: Option<Vec<u8>>,
    auxiliary_commands_bytes: Option<Vec<u8>>,
    auxiliary_archive_bytes: Option<Vec<u8>>,
    item_link_bytes: Option<Vec<u8>>,
    text_catalogs: Option<TextCatalogs>,
    text_catalog_bytes: Option<Vec<u8>>,
}

impl Dataset {
    fn files(&self) -> BaseFiles<'_> {
        BaseFiles {
            manifest: &self.manifest,
            dimensions: (&self.dimensions, &self.obj_bytes),
            background: Some((&self.background, &self.bg_bytes)),
            palettes: Some((&self.palettes, &self.palette_bytes)),
            alternate_palettes: Some((&self.alternate_palettes, &self.alternate_palette_bytes)),
            packet_programs: Some((&self.packet_programs, &self.packet_program_bytes)),
            main_graphics: self.main_graphics.as_ref().zip(self.main_graphic_bytes.as_deref()),
            transfers: self.transfers.as_ref().zip(self.transfer_bytes.as_deref()),
            short_texts: self.short_texts.as_ref().zip(self.short_text_bytes.as_deref()),
            font_bank: self.font_bank.as_ref().zip(self.font_bank_bytes.as_deref()),
            name_keyboard: self.name_keyboard.as_ref().zip(self.name_keyboard_bytes.as_deref()),
            obj_lists: self.obj_lists.as_ref().zip(self.obj_list_bytes.as_deref()),
            obj_animations: self.obj_animations.as_ref().zip(self.obj_animation_bytes.as_deref()),
            obj_graphics: self.obj_graphics.as_ref().zip(self.obj_graphic_bytes.as_deref()),
            ui_palettes: self.ui_palettes.as_ref().zip(self.ui_palette_bytes.as_deref()),
            ui_graphics: self.ui_graphics.as_ref().zip(self.ui_graphic_bytes.as_deref()),
            items: self.items.as_ref().zip(self.item_bytes.as_deref()),
            item_links: self.item_links.as_ref().zip(self.item_link_bytes.as_deref()),
            metadata16: self.metadata16.as_ref().zip(self.metadata16_bytes.as_deref()),
            metadata52: self.metadata52.as_ref().zip(self.metadata52_bytes.as_deref()),
            metadata4: self.metadata4.as_ref().zip(self.metadata4_bytes.as_deref()),
            metadata2: self.metadata2.as_ref().zip(self.metadata2_bytes.as_deref()),
            default48: self.default48.as_ref().zip(self.default48_bytes.as_deref()),
            constructor_alternates: self.constructor_alternates.as_ref().zip(self.constructor_alternates_bytes.as_deref()),
            metadata14: self.metadata14.as_ref().zip(self.metadata14_bytes.as_deref()),
            metadata12: self.metadata12.as_ref().zip(self.metadata12_bytes.as_deref()),
            global_pair_flags: self.global_pair_flags.as_ref().zip(self.global_pair_flags_bytes.as_deref()),
            evaluation_parameters: self.evaluation_parameters.as_ref().zip(self.evaluation_parameters_bytes.as_deref()),
            progression_rewards: self.progression_rewards.as_ref().zip(self.progression_rewards_bytes.as_deref()),
            progression_gates: self.progression_gates.as_ref().zip(self.progression_gates_bytes.as_deref()),
            progression_selection_gates: self.progression_selection_gates.as_ref().zip(self.progression_selection_gates_bytes.as_deref()),
            aggregate_flag_keys: self.aggregate_flag_keys.as_ref().zip(self.aggregate_flag_keys_bytes.as_deref()),
            trade_parameters: self.trade_parameters.as_ref().zip(self.trade_parameters_bytes.as_deref()),
            item_effect_parameters: self.item_effect_parameters.as_ref().zip(self.item_effect_parameters_bytes.as_deref()),
            audio_directory: self.audio_directory.as_ref().zip(self.audio_directory_bytes.as_deref()),
            audio_tones: self.audio_tones.as_ref().zip(self.audio_tones_bytes.as_deref()),
            audio_sequences: self.audio_sequences.as_ref().zip(self.audio_sequences_bytes.as_deref()),
            audio_driver: self.audio_driver.as_ref().zip(self.audio_driver_bytes.as_deref()),
            palette_effect_dispatch: self.palette_effect_dispatch.as_ref().zip(self.palette_effect_dispatch_bytes.as_deref()),
            palette_programs: self.palette_programs.as_ref().zip(self.palette_programs_bytes.as_deref()),
            blend_programs: self.blend_programs.as_ref().zip(self.blend_programs_bytes.as_deref()),
            task_descriptors: self.task_descriptors.as_ref().zip(self.task_descriptors_bytes.as_deref()),
            effect_event_directories: self.effect_event_directories.as_ref().zip(self.effect_event_directories_bytes.as_deref()),
            script_commands: self.script_commands.as_ref().zip(self.script_commands_bytes.as_deref()),
            script_archive: self.script_archive.as_ref().zip(self.script_archive_bytes.as_deref()),
            script_text: self.script_text.as_ref().zip(self.script_text_bytes.as_deref()),
            script_coordinates: self.script_coordinates.as_ref().zip(self.script_coordinates_bytes.as_deref()),
            ui_choice_lists: self.ui_choice_lists.as_ref().zip(self.ui_choice_lists_bytes.as_deref()),
            unit_obj_initial_pose: self.unit_obj_initial_pose.as_ref().zip(self.unit_obj_initial_pose_bytes.as_deref()),
            color_mode_palettes: self.color_mode_palettes.as_ref().zip(self.color_mode_palettes_bytes.as_deref()),
            unit_obj_equipment: self.unit_obj_equipment.as_ref().zip(self.unit_obj_equipment_bytes.as_deref()),
            unit_obj_normal_battle: self.unit_obj_normal_battle.as_ref().zip(self.unit_obj_normal_battle_bytes.as_deref()),
            unit_obj_alternate_battle: self.unit_obj_alternate_battle.as_ref().zip(self.unit_obj_alternate_battle_bytes.as_deref()),
            unit_obj_script_special_normal: self.unit_obj_script_special_normal.as_ref().zip(self.unit_obj_script_special_normal_bytes.as_deref()),
            unit_obj_script_special_alternate: self.unit_obj_script_special_alternate.as_ref().zip(self.unit_obj_script_special_alternate_bytes.as_deref()),
            unit_obj_script_override: self.unit_obj_script_override.as_ref().zip(self.unit_obj_script_override_bytes.as_deref()),
            unit_obj_script_registry_special: self.unit_obj_script_registry_special.as_ref().zip(self.unit_obj_script_registry_special_bytes.as_deref()),
            unit_obj_script_registry_generic: self.unit_obj_script_registry_generic.as_ref().zip(self.unit_obj_script_registry_generic_bytes.as_deref()),
            unit_obj_property_words: self.unit_obj_property_words.as_ref().zip(self.unit_obj_property_words_bytes.as_deref()),
            unit_obj_byte_property_words: self.unit_obj_byte_property_words.as_ref().zip(self.unit_obj_byte_property_words_bytes.as_deref()),
            unit_obj_ffff_reader_fields: self.unit_obj_ffff_reader_fields.as_ref().zip(self.unit_obj_ffff_reader_fields_bytes.as_deref()),
            slot_job_lookup: self.slot_job_lookup.as_ref().zip(self.slot_job_lookup_bytes.as_deref()),
            observed_task_parameters: self.observed_task_parameters.as_ref().zip(self.observed_task_parameters_bytes.as_deref()),
            constructor_obj_animation: self.constructor_obj_animation.as_ref().zip(self.constructor_obj_animation_bytes.as_deref()),
            task_cycle_deltas: self.task_cycle_deltas.as_ref().zip(self.task_cycle_deltas_bytes.as_deref()),
            word_task_obj_animations: self.word_task_obj_animations.as_ref().zip(self.word_task_obj_animations_bytes.as_deref()),
            world_nodes: self.world_nodes.as_ref().zip(self.world_nodes_bytes.as_deref()),
            world_roads: self.world_roads.as_ref().zip(self.world_roads_bytes.as_deref()),
            encounter_headers: self.encounter_headers.as_ref().zip(self.encounter_headers_bytes.as_deref()),
            encounter_unit_templates: self.encounter_unit_templates.as_ref().zip(self.encounter_unit_templates_bytes.as_deref()),
            encounter_coordinates: self.encounter_coordinates.as_ref().zip(self.encounter_coordinates_bytes.as_deref()),
            metadata28: self.metadata28.as_ref().zip(self.metadata28_bytes.as_deref()),
            records4: self.records4.as_ref().zip(self.records4_bytes.as_deref()),
            conditional_records8: self.conditional_records8.as_ref().zip(self.conditional_records8_bytes.as_deref()),
            mission_records70: self.mission_records70.as_ref().zip(self.mission_records70_bytes.as_deref()),
            selection_byte_tables: self.selection_byte_tables.as_ref().zip(self.selection_byte_tables_bytes.as_deref()),
            event_records12: self.event_records12.as_ref().zip(self.event_records12_bytes.as_deref()),
            parameter_records27: self.parameter_records27.as_ref().zip(self.parameter_records27_bytes.as_deref()),
            world_records38: self.world_records38.as_ref().zip(self.world_records38_bytes.as_deref()),
            indexed_graphics: self.indexed_graphics.as_ref().zip(self.indexed_graphics_bytes.as_deref()),
            ui_raw_transfers: self.ui_raw_transfers.as_ref().zip(self.ui_raw_transfers_bytes.as_deref()),
            dynamic_raw_fragments: self.dynamic_raw_fragments.as_ref().zip(self.dynamic_raw_fragments_bytes.as_deref()),
            conditional_four_byte_lookup: self.conditional_four_byte_lookup.as_ref().zip(self.conditional_four_byte_lookup_bytes.as_deref()),
            conditional_packed_lookup: self.conditional_packed_lookup.as_ref().zip(self.conditional_packed_lookup_bytes.as_deref()),
            conditional_descriptor_banks: self.conditional_descriptor_banks.as_ref().zip(self.conditional_descriptor_banks_bytes.as_deref()),
            conditional_descriptor_bank_zero: self.conditional_descriptor_bank_zero.as_ref().zip(self.conditional_descriptor_bank_zero_bytes.as_deref()),
            conditional_property_masks: self.conditional_property_masks.as_ref().zip(self.conditional_property_masks_bytes.as_deref()),
            compact_obj_animations: self.compact_obj_animations.as_ref().zip(self.compact_obj_animations_bytes.as_deref()),
            secondary_compact_obj_animations: self.secondary_compact_obj_animations.as_ref().zip(self.secondary_compact_obj_animations_bytes.as_deref()),
            tertiary_compact_obj_animations: self.tertiary_compact_obj_animations.as_ref().zip(self.tertiary_compact_obj_animations_bytes.as_deref()),
            quaternary_compact_obj_animations: self.quaternary_compact_obj_animations.as_ref().zip(self.quaternary_compact_obj_animations_bytes.as_deref()),
            quinary_compact_obj_animations: self.quinary_compact_obj_animations.as_ref().zip(self.quinary_compact_obj_animations_bytes.as_deref()),
            senary_compact_obj_animations: self.senary_compact_obj_animations.as_ref().zip(self.senary_compact_obj_animations_bytes.as_deref()),
            map_raster_palettes: self.map_raster_palettes.as_ref().zip(self.map_raster_palettes_bytes.as_deref()),
            auxiliary_commands: self.auxiliary_commands.as_ref().zip(self.auxiliary_commands_bytes.as_deref()),
            auxiliary_archive: self.auxiliary_archive.as_ref().zip(self.auxiliary_archive_bytes.as_deref()),
            text_catalogs: self.text_catalogs.as_ref().zip(self.text_catalog_bytes.as_deref()),
        }
    }
}

fn read_dataset(path: &Path) -> Result<Dataset, String> {
    if !fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir()) {
        return Err(format!(
            "Dataset must be a directory, not a symlink: {}",
            path.display()
        ));
    }
    let manifest_bytes = read_bounded(&path.join(MANIFEST_FILE), MAX_MANIFEST_FILE_BYTES)?;
    let manifest: Manifest = parse(&manifest_bytes, "resource manifest")?;
    // Never follow paths from a manifest. The supported resource set and names
    // are fixed, and each file has its own bound before parsing.
    let obj_bytes = read_bounded(&path.join(OBJ_DIMENSIONS_FILE), MAX_FILE_BYTES)?;
    let bg_bytes = read_bounded(
        &path.join(BG_DESCRIPTORS_FILE),
        MAX_BG_DESCRIPTORS_FILE_BYTES,
    )?;
    let palette_bytes = read_bounded(
        &path.join(BG_BASE_PALETTES_FILE),
        MAX_BG_BASE_PALETTES_FILE_BYTES,
    )?;
    let alternate_palette_bytes = read_bounded(
        &path.join(BG_ALTERNATE_PALETTES_FILE), MAX_BG_ALTERNATE_PALETTES_FILE_BYTES,
    )?;
    let packet_program_bytes = read_bounded(
        &path.join(BG_PACKET_PROGRAMS_FILE), MAX_BG_PACKET_PROGRAMS_FILE_BYTES,
    )?;
    // Presence selects only a fixed filename, never a manifest-provided path.
    // Authentication below rejects missing/unexpected resources against ROM.
    let main_graphic_bytes = manifest.coverage.iter().any(|id| id == BG_MAIN_GRAPHICS_ID)
        .then(|| read_bounded(&path.join(BG_MAIN_GRAPHICS_FILE), MAX_BG_MAIN_GRAPHICS_FILE_BYTES))
        .transpose()?;
    let transfer_bytes = manifest.coverage.iter().any(|id| id == BG_TRANSFER_PROGRAMS_ID)
        .then(|| read_bounded(&path.join(BG_TRANSFER_PROGRAMS_FILE), MAX_BG_TRANSFER_PROGRAMS_FILE_BYTES))
        .transpose()?;
    let short_text_bytes = manifest.coverage.iter().any(|id| id == SHORT_TEXTS_ID)
        .then(|| read_bounded(&path.join(SHORT_TEXTS_FILE), MAX_SHORT_TEXTS_FILE_BYTES))
        .transpose()?;
    let text_catalog_bytes = manifest.coverage.iter().any(|id| id == TEXT_CATALOGS_ID)
        .then(|| read_bounded(&path.join(TEXT_CATALOGS_FILE), MAX_TEXT_CATALOGS_FILE_BYTES)).transpose()?;
    let font_bank_bytes = manifest.coverage.iter().any(|id| id == FONT_BANK_ID)
        .then(|| read_bounded(&path.join(FONT_BANK_FILE), MAX_FONT_BANK_FILE_BYTES)).transpose()?;
    let name_keyboard_bytes = manifest.coverage.iter().any(|id| id == NAME_KEYBOARD_ID)
        .then(|| read_bounded(&path.join(NAME_KEYBOARD_FILE), MAX_NAME_KEYBOARD_FILE_BYTES)).transpose()?;
    let obj_list_bytes = manifest.coverage.iter().any(|id| id == OBJ_STATIC_LISTS_ID)
        .then(|| read_bounded(&path.join(OBJ_STATIC_LISTS_FILE), MAX_OBJ_STATIC_LISTS_FILE_BYTES)).transpose()?;
    let obj_animation_bytes = manifest.coverage.iter().any(|id| id == OBJ_ANIMATION_BANKS_ID)
        .then(|| read_bounded(&path.join(OBJ_ANIMATION_BANKS_FILE), MAX_OBJ_ANIMATION_BANKS_FILE_BYTES)).transpose()?;
    let obj_graphic_bytes = manifest.coverage.iter().any(|id| id == OBJ_CUSTOM_GRAPHICS_ID)
        .then(|| read_bounded(&path.join(OBJ_CUSTOM_GRAPHICS_FILE), MAX_OBJ_CUSTOM_GRAPHICS_FILE_BYTES)).transpose()?;
    let ui_palette_bytes = manifest.coverage.iter().any(|id| id == UI_RAW_PALETTES_ID)
        .then(|| read_bounded(&path.join(UI_RAW_PALETTES_FILE), MAX_UI_RAW_PALETTES_FILE_BYTES)).transpose()?;
    let ui_graphic_bytes = manifest.coverage.iter().any(|id| id == UI_CUSTOM_GRAPHICS_ID)
        .then(|| read_bounded(&path.join(UI_CUSTOM_GRAPHICS_FILE), MAX_UI_CUSTOM_GRAPHICS_FILE_BYTES)).transpose()?;
    let item_bytes = manifest.coverage.iter().any(|id|id == ITEM_RECORDS_ID)
        .then(||read_bounded(&path.join(ITEM_RECORDS_FILE),MAX_ITEM_RECORDS_FILE_BYTES)).transpose()?;
    let item_link_bytes=manifest.coverage.iter().any(|id|id==ITEM_LINKS_ID)
        .then(||read_bounded(&path.join(ITEM_LINKS_FILE),MAX_ITEM_LINKS_FILE_BYTES)).transpose()?;
    let metadata16_bytes=manifest.coverage.iter().any(|id|id==METADATA16_ID)
        .then(||read_bounded(&path.join(METADATA16_FILE),MAX_METADATA16_FILE_BYTES)).transpose()?;
    let metadata52_bytes=manifest.coverage.iter().any(|id|id==METADATA52_ID)
        .then(||read_bounded(&path.join(METADATA52_FILE),MAX_METADATA52_FILE_BYTES)).transpose()?;
    let metadata4_bytes=manifest.coverage.iter().any(|id|id==METADATA4_ID)
        .then(||read_bounded(&path.join(METADATA4_FILE),MAX_METADATA4_FILE_BYTES)).transpose()?;
    let metadata2_bytes=manifest.coverage.iter().any(|id|id==METADATA2_ID)
        .then(||read_bounded(&path.join(METADATA2_FILE),MAX_METADATA2_FILE_BYTES)).transpose()?;
    let default48_bytes=manifest.coverage.iter().any(|id|id==DEFAULT48_ID)
        .then(||read_bounded(&path.join(DEFAULT48_FILE),MAX_DEFAULT48_FILE_BYTES)).transpose()?;
    let constructor_alternates_bytes=manifest.coverage.iter().any(|id|id==CONSTRUCTOR_ALTERNATES_ID)
        .then(||read_bounded(&path.join(CONSTRUCTOR_ALTERNATES_FILE),MAX_CONSTRUCTOR_ALTERNATES_FILE_BYTES)).transpose()?;
    let metadata14_bytes=manifest.coverage.iter().any(|id|id==METADATA14_ID)
        .then(||read_bounded(&path.join(METADATA14_FILE),MAX_METADATA14_FILE_BYTES)).transpose()?;
    let metadata12_bytes=manifest.coverage.iter().any(|id|id==METADATA12_ID)
        .then(||read_bounded(&path.join(METADATA12_FILE),MAX_METADATA12_FILE_BYTES)).transpose()?;
    let global_pair_flags_bytes=manifest.coverage.iter().any(|id|id==GLOBAL_PAIR_FLAGS_ID)
        .then(||read_bounded(&path.join(GLOBAL_PAIR_FLAGS_FILE),MAX_GLOBAL_PAIR_FLAGS_FILE_BYTES)).transpose()?;
    let evaluation_parameters_bytes=manifest.coverage.iter().any(|id|id==EVALUATION_PARAMETERS_ID)
        .then(||read_bounded(&path.join(EVALUATION_PARAMETERS_FILE),MAX_EVALUATION_PARAMETERS_FILE_BYTES)).transpose()?;
    let progression_rewards_bytes=manifest.coverage.iter().any(|id|id==PROGRESSION_REWARDS_ID)
        .then(||read_bounded(&path.join(PROGRESSION_REWARDS_FILE),MAX_PROGRESSION_REWARDS_FILE_BYTES)).transpose()?;
    let progression_gates_bytes=manifest.coverage.iter().any(|id|id==PROGRESSION_GATES_ID)
        .then(||read_bounded(&path.join(PROGRESSION_GATES_FILE),MAX_PROGRESSION_GATES_FILE_BYTES)).transpose()?;
    let progression_selection_gates_bytes=manifest.coverage.iter().any(|id|id==PROGRESSION_SELECTION_GATES_ID)
        .then(||read_bounded(&path.join(PROGRESSION_SELECTION_GATES_FILE),MAX_PROGRESSION_SELECTION_GATES_FILE_BYTES)).transpose()?;
    let aggregate_flag_keys_bytes=manifest.coverage.iter().any(|id|id==AGGREGATE_FLAG_KEYS_ID)
        .then(||read_bounded(&path.join(AGGREGATE_FLAG_KEYS_FILE),MAX_AGGREGATE_FLAG_KEYS_FILE_BYTES)).transpose()?;
    let trade_parameters_bytes=manifest.coverage.iter().any(|id|id==TRADE_PARAMETERS_ID)
        .then(||read_bounded(&path.join(TRADE_PARAMETERS_FILE),MAX_TRADE_PARAMETERS_FILE_BYTES)).transpose()?;
    let item_effect_parameters_bytes=manifest.coverage.iter().any(|id|id==ITEM_EFFECT_PARAMETERS_ID)
        .then(||read_bounded(&path.join(ITEM_EFFECT_PARAMETERS_FILE),MAX_ITEM_EFFECT_PARAMETERS_FILE_BYTES)).transpose()?;
    let audio_directory_bytes=manifest.coverage.iter().any(|id|id==AUDIO_DIRECTORY_ID)
        .then(||read_bounded(&path.join(AUDIO_DIRECTORY_FILE),MAX_AUDIO_DIRECTORY_FILE_BYTES)).transpose()?;
    let audio_tones_bytes=manifest.coverage.iter().any(|id|id==AUDIO_TONES_ID)
        .then(||read_bounded(&path.join(AUDIO_TONES_FILE),MAX_AUDIO_TONES_FILE_BYTES)).transpose()?;
    let audio_sequences_bytes=manifest.coverage.iter().any(|id|id==AUDIO_SEQUENCES_ID)
        .then(||read_bounded(&path.join(AUDIO_SEQUENCES_FILE),MAX_AUDIO_SEQUENCES_FILE_BYTES)).transpose()?;
    let audio_driver_bytes=manifest.coverage.iter().any(|id|id==AUDIO_DRIVER_ID)
        .then(||read_bounded(&path.join(AUDIO_DRIVER_FILE),MAX_AUDIO_DRIVER_FILE_BYTES)).transpose()?;
    let palette_effect_dispatch_bytes=manifest.coverage.iter().any(|id|id==PALETTE_EFFECT_DISPATCH_ID)
        .then(||read_bounded(&path.join(PALETTE_EFFECT_DISPATCH_FILE),MAX_PALETTE_EFFECT_DISPATCH_FILE_BYTES)).transpose()?;
    let palette_programs_bytes=manifest.coverage.iter().any(|id|id==PALETTE_PROGRAMS_ID)
        .then(||read_bounded(&path.join(PALETTE_PROGRAMS_FILE),MAX_PALETTE_PROGRAMS_FILE_BYTES)).transpose()?;
    let blend_programs_bytes=manifest.coverage.iter().any(|id|id==BLEND_PROGRAMS_ID)
        .then(||read_bounded(&path.join(BLEND_PROGRAMS_FILE),MAX_BLEND_PROGRAMS_FILE_BYTES)).transpose()?;
    let task_descriptors_bytes=manifest.coverage.iter().any(|id|id==TASK_DESCRIPTORS_ID)
        .then(||read_bounded(&path.join(TASK_DESCRIPTORS_FILE),MAX_TASK_DESCRIPTORS_FILE_BYTES)).transpose()?;
    let effect_event_directories_bytes=manifest.coverage.iter().any(|id|id==EFFECT_EVENT_DIRECTORIES_ID)
        .then(||read_bounded(&path.join(EFFECT_EVENT_DIRECTORIES_FILE),MAX_EFFECT_EVENT_DIRECTORIES_FILE_BYTES)).transpose()?;
    let script_commands_bytes=manifest.coverage.iter().any(|id|id==SCRIPT_COMMANDS_ID)
        .then(||read_bounded(&path.join(SCRIPT_COMMANDS_FILE),MAX_SCRIPT_COMMANDS_FILE_BYTES)).transpose()?;
    let script_archive_bytes=manifest.coverage.iter().any(|id|id==SCRIPT_ARCHIVE_ID)
        .then(||read_bounded(&path.join(SCRIPT_ARCHIVE_FILE),MAX_SCRIPT_ARCHIVE_FILE_BYTES)).transpose()?;
    let script_text_bytes=manifest.coverage.iter().any(|id|id==SCRIPT_TEXT_ID)
        .then(||read_bounded(&path.join(SCRIPT_TEXT_FILE),MAX_SCRIPT_TEXT_FILE_BYTES)).transpose()?;
    let script_coordinates_bytes=manifest.coverage.iter().any(|id|id==SCRIPT_COORDINATES_ID)
        .then(||read_bounded(&path.join(SCRIPT_COORDINATES_FILE),MAX_SCRIPT_COORDINATES_FILE_BYTES)).transpose()?;
    let ui_choice_lists_bytes=manifest.coverage.iter().any(|id|id==UI_CHOICE_LISTS_ID)
        .then(||read_bounded(&path.join(UI_CHOICE_LISTS_FILE),MAX_UI_CHOICE_LISTS_FILE_BYTES)).transpose()?;
    let unit_obj_initial_pose_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_INITIAL_POSE_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_INITIAL_POSE_FILE),MAX_UNIT_OBJ_INITIAL_POSE_FILE_BYTES)).transpose()?;
    let color_mode_palettes_bytes=manifest.coverage.iter().any(|id|id==COLOR_MODE_PALETTES_ID)
        .then(||read_bounded(&path.join(COLOR_MODE_PALETTES_FILE),MAX_COLOR_MODE_PALETTES_FILE_BYTES)).transpose()?;
    let unit_obj_equipment_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_EQUIPMENT_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_EQUIPMENT_FILE),MAX_UNIT_OBJ_EQUIPMENT_FILE_BYTES)).transpose()?;
    let unit_obj_normal_battle_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_NORMAL_BATTLE_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_NORMAL_BATTLE_FILE),MAX_UNIT_OBJ_NORMAL_BATTLE_FILE_BYTES)).transpose()?;
    let unit_obj_alternate_battle_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_ALTERNATE_BATTLE_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_ALTERNATE_BATTLE_FILE),MAX_UNIT_OBJ_ALTERNATE_BATTLE_FILE_BYTES)).transpose()?;
    let unit_obj_script_special_normal_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FILE),MAX_UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FILE_BYTES)).transpose()?;
    let unit_obj_script_special_alternate_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FILE),MAX_UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FILE_BYTES)).transpose()?;
    let unit_obj_script_override_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_SCRIPT_OVERRIDE_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_SCRIPT_OVERRIDE_FILE),MAX_UNIT_OBJ_SCRIPT_OVERRIDE_FILE_BYTES)).transpose()?;
    let unit_obj_script_registry_special_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FILE),MAX_UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FILE_BYTES)).transpose()?;
    let unit_obj_script_registry_generic_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FILE),MAX_UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FILE_BYTES)).transpose()?;
    let unit_obj_property_words_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_PROPERTY_WORDS_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_PROPERTY_WORDS_FILE),MAX_UNIT_OBJ_PROPERTY_WORDS_FILE_BYTES)).transpose()?;
    let unit_obj_byte_property_words_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_BYTE_PROPERTY_WORDS_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_BYTE_PROPERTY_WORDS_FILE),MAX_UNIT_OBJ_BYTE_PROPERTY_WORDS_FILE_BYTES)).transpose()?;
    let unit_obj_ffff_reader_fields_bytes=manifest.coverage.iter().any(|id|id==UNIT_OBJ_FFFF_READER_FIELDS_ID)
        .then(||read_bounded(&path.join(UNIT_OBJ_FFFF_READER_FIELDS_FILE),MAX_UNIT_OBJ_FFFF_READER_FIELDS_FILE_BYTES)).transpose()?;
    let slot_job_lookup_bytes=manifest.coverage.iter().any(|id|id==SLOT_JOB_LOOKUP_ID)
        .then(||read_bounded(&path.join(SLOT_JOB_LOOKUP_FILE),MAX_SLOT_JOB_LOOKUP_FILE_BYTES)).transpose()?;
    let observed_task_parameters_bytes=manifest.coverage.iter().any(|id|id==OBSERVED_TASK_PARAMETERS_ID)
        .then(||read_bounded(&path.join(OBSERVED_TASK_PARAMETERS_FILE),MAX_OBSERVED_TASK_PARAMETERS_FILE_BYTES)).transpose()?;
    let constructor_obj_animation_bytes=manifest.coverage.iter().any(|id|id==CONSTRUCTOR_OBJ_ANIMATION_ID)
        .then(||read_bounded(&path.join(CONSTRUCTOR_OBJ_ANIMATION_FILE),MAX_CONSTRUCTOR_OBJ_ANIMATION_FILE_BYTES)).transpose()?;
    let task_cycle_deltas_bytes=manifest.coverage.iter().any(|id|id==TASK_CYCLE_DELTAS_ID)
        .then(||read_bounded(&path.join(TASK_CYCLE_DELTAS_FILE),MAX_TASK_CYCLE_DELTAS_FILE_BYTES)).transpose()?;
    let ui_raw_transfers_bytes=manifest.coverage.iter().any(|id|id==UI_RAW_TRANSFERS_ID).then(||read_bounded(&path.join(UI_RAW_TRANSFERS_FILE),MAX_UI_RAW_TRANSFERS_FILE_BYTES)).transpose()?;
    let dynamic_raw_fragments_bytes=manifest.coverage.iter().any(|id|id==DYNAMIC_RAW_FRAGMENTS_ID).then(||read_bounded(&path.join(DYNAMIC_RAW_FRAGMENTS_FILE),MAX_DYNAMIC_RAW_FRAGMENTS_FILE_BYTES)).transpose()?;
    let conditional_four_byte_lookup_bytes=manifest.coverage.iter().any(|id|id==CONDITIONAL_FOUR_BYTE_LOOKUP_ID).then(||read_bounded(&path.join(CONDITIONAL_FOUR_BYTE_LOOKUP_FILE),MAX_CONDITIONAL_FOUR_BYTE_LOOKUP_FILE_BYTES)).transpose()?;
    let conditional_packed_lookup_bytes=manifest.coverage.iter().any(|id|id==CONDITIONAL_PACKED_LOOKUP_ID).then(||read_bounded(&path.join(CONDITIONAL_PACKED_LOOKUP_FILE),MAX_CONDITIONAL_PACKED_LOOKUP_FILE_BYTES)).transpose()?;
    let conditional_descriptor_banks_bytes=manifest.coverage.iter().any(|id|id==CONDITIONAL_DESCRIPTOR_BANKS_ID).then(||read_bounded(&path.join(CONDITIONAL_DESCRIPTOR_BANKS_FILE),MAX_CONDITIONAL_DESCRIPTOR_BANKS_FILE_BYTES)).transpose()?;
    let conditional_descriptor_bank_zero_bytes=manifest.coverage.iter().any(|id|id==CONDITIONAL_DESCRIPTOR_BANK_ZERO_ID).then(||read_bounded(&path.join(CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE),MAX_CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE_BYTES)).transpose()?;
    let conditional_property_masks_bytes=manifest.coverage.iter().any(|id|id==CONDITIONAL_PROPERTY_MASKS_ID).then(||read_bounded(&path.join(CONDITIONAL_PROPERTY_MASKS_FILE),MAX_CONDITIONAL_PROPERTY_MASKS_FILE_BYTES)).transpose()?;
    let compact_obj_animations_bytes=manifest.coverage.iter().any(|id|id==COMPACT_OBJ_ANIMATIONS_ID).then(||read_bounded(&path.join(COMPACT_OBJ_ANIMATIONS_FILE),MAX_COMPACT_OBJ_ANIMATIONS_FILE_BYTES)).transpose()?;
    let secondary_compact_obj_animations_bytes=manifest.coverage.iter().any(|id|id==SECONDARY_COMPACT_OBJ_ANIMATIONS_ID).then(||read_bounded(&path.join(SECONDARY_COMPACT_OBJ_ANIMATIONS_FILE),MAX_SECONDARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES)).transpose()?;
    let tertiary_compact_obj_animations_bytes=manifest.coverage.iter().any(|id|id==TERTIARY_COMPACT_OBJ_ANIMATIONS_ID).then(||read_bounded(&path.join(TERTIARY_COMPACT_OBJ_ANIMATIONS_FILE),MAX_TERTIARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES)).transpose()?;
    let quaternary_compact_obj_animations_bytes=manifest.coverage.iter().any(|id|id==QUATERNARY_COMPACT_OBJ_ANIMATIONS_ID).then(||read_bounded(&path.join(QUATERNARY_COMPACT_OBJ_ANIMATIONS_FILE),MAX_QUATERNARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES)).transpose()?;
    let quinary_compact_obj_animations_bytes=manifest.coverage.iter().any(|id|id==QUINARY_COMPACT_OBJ_ANIMATIONS_ID).then(||read_bounded(&path.join(QUINARY_COMPACT_OBJ_ANIMATIONS_FILE),MAX_QUINARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES)).transpose()?;
    let senary_compact_obj_animations_bytes=manifest.coverage.iter().any(|id|id==SENARY_COMPACT_OBJ_ANIMATIONS_ID).then(||read_bounded(&path.join(SENARY_COMPACT_OBJ_ANIMATIONS_FILE),MAX_SENARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES)).transpose()?;
    let indexed_graphics_bytes=manifest.coverage.iter().any(|id|id==INDEXED_GRAPHICS_ID).then(||read_bounded(&path.join(INDEXED_GRAPHICS_FILE),MAX_INDEXED_GRAPHICS_FILE_BYTES)).transpose()?;
    let world_records38_bytes=manifest.coverage.iter().any(|id|id==WORLD_RECORDS38_ID).then(||read_bounded(&path.join(WORLD_RECORDS38_FILE),MAX_WORLD_RECORDS38_FILE_BYTES)).transpose()?;
    let parameter_records27_bytes=manifest.coverage.iter().any(|id|id==PARAMETER_RECORDS27_ID).then(||read_bounded(&path.join(PARAMETER_RECORDS27_FILE),MAX_PARAMETER_RECORDS27_FILE_BYTES)).transpose()?;
    let event_records12_bytes=manifest.coverage.iter().any(|id|id==EVENT_RECORDS12_ID).then(||read_bounded(&path.join(EVENT_RECORDS12_FILE),MAX_EVENT_RECORDS12_FILE_BYTES)).transpose()?;
    let selection_byte_tables_bytes=manifest.coverage.iter().any(|id|id==SELECTION_BYTE_TABLES_ID).then(||read_bounded(&path.join(SELECTION_BYTE_TABLES_FILE),MAX_SELECTION_BYTE_TABLES_FILE_BYTES)).transpose()?;
    let mission_records70_bytes=manifest.coverage.iter().any(|id|id==MISSION_RECORDS70_ID).then(||read_bounded(&path.join(MISSION_RECORDS70_FILE),MAX_MISSION_RECORDS70_FILE_BYTES)).transpose()?;
    let conditional_records8_bytes=manifest.coverage.iter().any(|id|id==CONDITIONAL_RECORDS8_ID).then(||read_bounded(&path.join(CONDITIONAL_RECORDS8_FILE),MAX_CONDITIONAL_RECORDS8_FILE_BYTES)).transpose()?;
    let records4_bytes=manifest.coverage.iter().any(|id|id==RECORDS4_ID).then(||read_bounded(&path.join(RECORDS4_FILE),MAX_RECORDS4_FILE_BYTES)).transpose()?;
    let metadata28_bytes=manifest.coverage.iter().any(|id|id==METADATA28_ID).then(||read_bounded(&path.join(METADATA28_FILE),MAX_METADATA28_FILE_BYTES)).transpose()?;
    let encounter_coordinates_bytes=manifest.coverage.iter().any(|id|id==ENCOUNTER_COORDINATES_ID).then(||read_bounded(&path.join(ENCOUNTER_COORDINATES_FILE),MAX_ENCOUNTER_COORDINATES_FILE_BYTES)).transpose()?;
    let encounter_unit_templates_bytes=manifest.coverage.iter().any(|id|id==ENCOUNTER_UNIT_TEMPLATES_ID)
        .then(||read_bounded(&path.join(ENCOUNTER_UNIT_TEMPLATES_FILE),MAX_ENCOUNTER_UNIT_TEMPLATES_FILE_BYTES)).transpose()?;
    let encounter_headers_bytes=manifest.coverage.iter().any(|id|id==ENCOUNTER_HEADERS_ID)
        .then(||read_bounded(&path.join(ENCOUNTER_HEADERS_FILE),MAX_ENCOUNTER_HEADERS_FILE_BYTES)).transpose()?;
    let world_roads_bytes=manifest.coverage.iter().any(|id|id==WORLD_ROADS_ID)
        .then(||read_bounded(&path.join(WORLD_ROADS_FILE),MAX_WORLD_ROADS_FILE_BYTES)).transpose()?;
    let world_nodes_bytes=manifest.coverage.iter().any(|id|id==WORLD_NODES_ID)
        .then(||read_bounded(&path.join(WORLD_NODES_FILE),MAX_WORLD_NODES_FILE_BYTES)).transpose()?;
    let word_task_obj_animations_bytes=manifest.coverage.iter().any(|id|id==WORD_TASK_OBJ_ANIMATIONS_ID)
        .then(||read_bounded(&path.join(WORD_TASK_OBJ_ANIMATIONS_FILE),MAX_WORD_TASK_OBJ_ANIMATIONS_FILE_BYTES)).transpose()?;
    let map_raster_palettes_bytes=manifest.coverage.iter().any(|id|id==MAP_RASTER_PALETTES_ID)
        .then(||read_bounded(&path.join(MAP_RASTER_PALETTES_FILE),MAX_MAP_RASTER_PALETTES_FILE_BYTES)).transpose()?;
    let auxiliary_commands_bytes=manifest.coverage.iter().any(|id|id==AUXILIARY_COMMANDS_ID)
        .then(||read_bounded(&path.join(AUXILIARY_COMMANDS_FILE),MAX_AUXILIARY_COMMANDS_FILE_BYTES)).transpose()?;
    let auxiliary_archive_bytes=manifest.coverage.iter().any(|id|id==AUXILIARY_ARCHIVE_ID)
        .then(||read_bounded(&path.join(AUXILIARY_ARCHIVE_FILE),MAX_AUXILIARY_ARCHIVE_FILE_BYTES)).transpose()?;
    check_dataset_budget(
        manifest_bytes.len(),
        obj_bytes.len(),
        bg_bytes.len(),
        palette_bytes.len(),
        alternate_palette_bytes.len(),
        packet_program_bytes.len(),
        main_graphic_bytes.as_ref().map_or(0, Vec::len),
        transfer_bytes.as_ref().map_or(0, Vec::len),
        short_text_bytes.as_ref().map_or(0, Vec::len),
        text_catalog_bytes.as_ref().map_or(0, Vec::len),
        font_bank_bytes.as_ref().map_or(0, Vec::len),
        name_keyboard_bytes.as_ref().map_or(0, Vec::len),
        obj_list_bytes.as_ref().map_or(0, Vec::len),
        obj_animation_bytes.as_ref().map_or(0, Vec::len),
        obj_graphic_bytes.as_ref().map_or(0, Vec::len),
        ui_palette_bytes.as_ref().map_or(0, Vec::len),
        ui_graphic_bytes.as_ref().map_or(0, Vec::len),
        item_bytes.as_ref().map_or(0, Vec::len),
        item_link_bytes.as_ref().map_or(0, Vec::len),
        metadata16_bytes.as_ref().map_or(0, Vec::len),
        metadata52_bytes.as_ref().map_or(0, Vec::len),
        metadata4_bytes.as_ref().map_or(0, Vec::len),
        metadata2_bytes.as_ref().map_or(0, Vec::len),
        default48_bytes.as_ref().map_or(0, Vec::len),
        constructor_alternates_bytes.as_ref().map_or(0, Vec::len),
        metadata14_bytes.as_ref().map_or(0, Vec::len),
        metadata12_bytes.as_ref().map_or(0, Vec::len),
        global_pair_flags_bytes.as_ref().map_or(0, Vec::len),
        evaluation_parameters_bytes.as_ref().map_or(0, Vec::len),
        progression_rewards_bytes.as_ref().map_or(0, Vec::len),
        progression_gates_bytes.as_ref().map_or(0, Vec::len),
        progression_selection_gates_bytes.as_ref().map_or(0, Vec::len),
        aggregate_flag_keys_bytes.as_ref().map_or(0, Vec::len),
        trade_parameters_bytes.as_ref().map_or(0, Vec::len),
        item_effect_parameters_bytes.as_ref().map_or(0, Vec::len),
        audio_directory_bytes.as_ref().map_or(0, Vec::len),
        audio_tones_bytes.as_ref().map_or(0, Vec::len),
        audio_sequences_bytes.as_ref().map_or(0, Vec::len),
        audio_driver_bytes.as_ref().map_or(0, Vec::len),
        palette_effect_dispatch_bytes.as_ref().map_or(0, Vec::len),
        palette_programs_bytes.as_ref().map_or(0, Vec::len),
        blend_programs_bytes.as_ref().map_or(0, Vec::len),
        task_descriptors_bytes.as_ref().map_or(0, Vec::len),
        effect_event_directories_bytes.as_ref().map_or(0, Vec::len),
        script_commands_bytes.as_ref().map_or(0, Vec::len),
        script_archive_bytes.as_ref().map_or(0, Vec::len),
        script_text_bytes.as_ref().map_or(0, Vec::len),
        script_coordinates_bytes.as_ref().map_or(0, Vec::len),
        ui_choice_lists_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_initial_pose_bytes.as_ref().map_or(0, Vec::len),
        color_mode_palettes_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_equipment_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_normal_battle_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_alternate_battle_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_script_special_normal_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_script_special_alternate_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_script_override_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_script_registry_special_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_script_registry_generic_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_property_words_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_byte_property_words_bytes.as_ref().map_or(0, Vec::len),
        unit_obj_ffff_reader_fields_bytes.as_ref().map_or(0, Vec::len),
        slot_job_lookup_bytes.as_ref().map_or(0, Vec::len),
        observed_task_parameters_bytes.as_ref().map_or(0, Vec::len),
        constructor_obj_animation_bytes.as_ref().map_or(0, Vec::len),
        task_cycle_deltas_bytes.as_ref().map_or(0, Vec::len),
        word_task_obj_animations_bytes.as_ref().map_or(0, Vec::len),
        world_nodes_bytes.as_ref().map_or(0, Vec::len),
        world_roads_bytes.as_ref().map_or(0, Vec::len),
        encounter_headers_bytes.as_ref().map_or(0, Vec::len),
        encounter_unit_templates_bytes.as_ref().map_or(0, Vec::len),
        encounter_coordinates_bytes.as_ref().map_or(0, Vec::len),
        metadata28_bytes.as_ref().map_or(0, Vec::len),
        records4_bytes.as_ref().map_or(0, Vec::len),
        conditional_records8_bytes.as_ref().map_or(0, Vec::len),
        mission_records70_bytes.as_ref().map_or(0, Vec::len),
        selection_byte_tables_bytes.as_ref().map_or(0, Vec::len),
        event_records12_bytes.as_ref().map_or(0, Vec::len),
        parameter_records27_bytes.as_ref().map_or(0, Vec::len),
        world_records38_bytes.as_ref().map_or(0, Vec::len),
        indexed_graphics_bytes.as_ref().map_or(0, Vec::len),
        ui_raw_transfers_bytes.as_ref().map_or(0, Vec::len),
        dynamic_raw_fragments_bytes.as_ref().map_or(0, Vec::len),
        conditional_four_byte_lookup_bytes.as_ref().map_or(0, Vec::len),
        conditional_packed_lookup_bytes.as_ref().map_or(0, Vec::len),
        conditional_descriptor_banks_bytes.as_ref().map_or(0, Vec::len),
        conditional_descriptor_bank_zero_bytes.as_ref().map_or(0, Vec::len),
        conditional_property_masks_bytes.as_ref().map_or(0, Vec::len),
        compact_obj_animations_bytes.as_ref().map_or(0, Vec::len),
        secondary_compact_obj_animations_bytes.as_ref().map_or(0, Vec::len),
        tertiary_compact_obj_animations_bytes.as_ref().map_or(0, Vec::len),
        quaternary_compact_obj_animations_bytes.as_ref().map_or(0, Vec::len),
        quinary_compact_obj_animations_bytes.as_ref().map_or(0, Vec::len),
        senary_compact_obj_animations_bytes.as_ref().map_or(0, Vec::len),
        map_raster_palettes_bytes.as_ref().map_or(0, Vec::len),
        auxiliary_commands_bytes.as_ref().map_or(0, Vec::len),
        auxiliary_archive_bytes.as_ref().map_or(0, Vec::len),
    )?;
    let dimensions = parse(&obj_bytes, "OBJ dimensions")?;
    let background = parse_background(&bg_bytes)?;
    let palettes = parse_palettes(&palette_bytes)?;
    let alternate_palettes = parse_alternate_palettes(&alternate_palette_bytes)?;
    let packet_programs = parse_packet_programs(&packet_program_bytes)?;
    let main_graphics = main_graphic_bytes.as_deref().map(parse_main_graphics).transpose()?;
    let transfers = transfer_bytes.as_deref().map(parse_transfers).transpose()?;
    let short_texts = short_text_bytes.as_deref().map(parse_short_texts).transpose()?;
    let text_catalogs = text_catalog_bytes.as_deref().map(parse_text_catalogs).transpose()?;
    let font_bank = font_bank_bytes.as_deref().map(parse_font_bank).transpose()?;
    let name_keyboard = name_keyboard_bytes.as_deref().map(parse_name_keyboard).transpose()?;
    let obj_lists = obj_list_bytes.as_deref().map(parse_obj_lists).transpose()?;
    let obj_animations = obj_animation_bytes.as_deref().map(parse_obj_animations).transpose()?;
    let obj_graphics = obj_graphic_bytes.as_deref().map(parse_obj_graphics).transpose()?;
    let ui_palettes = ui_palette_bytes.as_deref().map(parse_ui_palettes).transpose()?;
    let ui_graphics = ui_graphic_bytes.as_deref().map(parse_ui_graphics).transpose()?;
    let items = item_bytes.as_deref().map(parse_items).transpose()?;
    let item_links=item_link_bytes.as_deref().map(parse_item_links).transpose()?;
    let metadata16=metadata16_bytes.as_deref().map(parse_metadata16).transpose()?;
    let metadata52=metadata52_bytes.as_deref().map(parse_metadata52).transpose()?;
    let metadata4=metadata4_bytes.as_deref().map(parse_metadata4).transpose()?;
    let metadata2=metadata2_bytes.as_deref().map(parse_metadata2).transpose()?;
    let default48=default48_bytes.as_deref().map(parse_default48).transpose()?;
    let constructor_alternates=constructor_alternates_bytes.as_deref().map(parse_constructor_alternates).transpose()?;
    let metadata14=metadata14_bytes.as_deref().map(parse_metadata14).transpose()?;
    let metadata12=metadata12_bytes.as_deref().map(parse_metadata12).transpose()?;
    let global_pair_flags=global_pair_flags_bytes.as_deref().map(parse_global_pair_flags).transpose()?;
    let evaluation_parameters=evaluation_parameters_bytes.as_deref().map(parse_evaluation_parameters).transpose()?;
    let progression_rewards=progression_rewards_bytes.as_deref().map(parse_progression_rewards).transpose()?;
    let progression_gates=progression_gates_bytes.as_deref().map(parse_progression_gates).transpose()?;
    let progression_selection_gates=progression_selection_gates_bytes.as_deref().map(parse_progression_selection_gates).transpose()?;
    let aggregate_flag_keys=aggregate_flag_keys_bytes.as_deref().map(parse_aggregate_flag_keys).transpose()?;
    let trade_parameters=trade_parameters_bytes.as_deref().map(parse_trade_parameters).transpose()?;
    let item_effect_parameters=item_effect_parameters_bytes.as_deref().map(parse_item_effect_parameters).transpose()?;
    let audio_directory=audio_directory_bytes.as_deref().map(parse_audio_directory).transpose()?;
    let audio_tones=audio_tones_bytes.as_deref().map(parse_audio_tones).transpose()?;
    let audio_sequences=audio_sequences_bytes.as_deref().map(parse_audio_sequences).transpose()?;
    let audio_driver=audio_driver_bytes.as_deref().map(parse_audio_driver).transpose()?;
    let palette_effect_dispatch=palette_effect_dispatch_bytes.as_deref().map(parse_palette_effect_dispatch).transpose()?;
    let palette_programs=palette_programs_bytes.as_deref().map(parse_palette_programs).transpose()?;
    let blend_programs=blend_programs_bytes.as_deref().map(parse_blend_programs).transpose()?;
    let task_descriptors=task_descriptors_bytes.as_deref().map(parse_task_descriptors).transpose()?;
    let effect_event_directories=effect_event_directories_bytes.as_deref().map(parse_effect_event_directories).transpose()?;
    let script_commands=script_commands_bytes.as_deref().map(parse_script_commands).transpose()?;
    let script_archive=script_archive_bytes.as_deref().map(parse_script_archive).transpose()?;
    let script_text=script_text_bytes.as_deref().map(parse_script_text).transpose()?;
    let script_coordinates=script_coordinates_bytes.as_deref().map(parse_script_coordinates).transpose()?;
    let ui_choice_lists=ui_choice_lists_bytes.as_deref().map(parse_ui_choice_lists).transpose()?;
    let unit_obj_initial_pose=unit_obj_initial_pose_bytes.as_deref().map(parse_unit_obj_initial_pose).transpose()?;
    let color_mode_palettes=color_mode_palettes_bytes.as_deref().map(parse_color_mode_palettes).transpose()?;
    let unit_obj_equipment=unit_obj_equipment_bytes.as_deref().map(parse_unit_obj_equipment).transpose()?;
    let unit_obj_normal_battle=unit_obj_normal_battle_bytes.as_deref().map(parse_unit_obj_normal_battle).transpose()?;
    let unit_obj_alternate_battle=unit_obj_alternate_battle_bytes.as_deref().map(parse_unit_obj_alternate_battle).transpose()?;
    let unit_obj_script_special_normal=unit_obj_script_special_normal_bytes.as_deref().map(parse_unit_obj_script_special_normal).transpose()?;
    let unit_obj_script_special_alternate=unit_obj_script_special_alternate_bytes.as_deref().map(parse_unit_obj_script_special_alternate).transpose()?;
    let unit_obj_script_override=unit_obj_script_override_bytes.as_deref().map(parse_unit_obj_script_override).transpose()?;
    let unit_obj_script_registry_special=unit_obj_script_registry_special_bytes.as_deref().map(parse_unit_obj_script_registry_special).transpose()?;
    let unit_obj_script_registry_generic=unit_obj_script_registry_generic_bytes.as_deref().map(parse_unit_obj_script_registry_generic).transpose()?;
    let unit_obj_property_words=unit_obj_property_words_bytes.as_deref().map(parse_unit_obj_property_words).transpose()?;
    let unit_obj_byte_property_words=unit_obj_byte_property_words_bytes.as_deref().map(parse_unit_obj_byte_property_words).transpose()?;
    let unit_obj_ffff_reader_fields=unit_obj_ffff_reader_fields_bytes.as_deref().map(parse_unit_obj_ffff_reader_fields).transpose()?;
    let slot_job_lookup=slot_job_lookup_bytes.as_deref().map(parse_slot_job_lookup).transpose()?;
    let observed_task_parameters=observed_task_parameters_bytes.as_deref().map(parse_observed_task_parameters).transpose()?;
    let constructor_obj_animation=constructor_obj_animation_bytes.as_deref().map(parse_constructor_obj_animation).transpose()?;
    let task_cycle_deltas=task_cycle_deltas_bytes.as_deref().map(parse_task_cycle_deltas).transpose()?;
    let ui_raw_transfers=ui_raw_transfers_bytes.as_deref().map(parse_ui_raw_transfers).transpose()?;
    let dynamic_raw_fragments=dynamic_raw_fragments_bytes.as_deref().map(parse_dynamic_raw_fragments).transpose()?;
    let conditional_four_byte_lookup=conditional_four_byte_lookup_bytes.as_deref().map(parse_conditional_four_byte_lookup).transpose()?;
    let conditional_packed_lookup=conditional_packed_lookup_bytes.as_deref().map(parse_conditional_packed_lookup).transpose()?;
    let conditional_descriptor_banks=conditional_descriptor_banks_bytes.as_deref().map(parse_conditional_descriptor_banks).transpose()?;
    let conditional_descriptor_bank_zero=conditional_descriptor_bank_zero_bytes.as_deref().map(parse_conditional_descriptor_bank_zero).transpose()?;
    let conditional_property_masks=conditional_property_masks_bytes.as_deref().map(parse_conditional_property_masks).transpose()?;
    let compact_obj_animations=compact_obj_animations_bytes.as_deref().map(parse_compact_obj_animations).transpose()?;
    let secondary_compact_obj_animations=secondary_compact_obj_animations_bytes.as_deref().map(parse_secondary_compact_obj_animations).transpose()?;
    let tertiary_compact_obj_animations=tertiary_compact_obj_animations_bytes.as_deref().map(parse_tertiary_compact_obj_animations).transpose()?;
    let quaternary_compact_obj_animations=quaternary_compact_obj_animations_bytes.as_deref().map(parse_quaternary_compact_obj_animations).transpose()?;
    let quinary_compact_obj_animations=quinary_compact_obj_animations_bytes.as_deref().map(parse_quinary_compact_obj_animations).transpose()?;
    let senary_compact_obj_animations=senary_compact_obj_animations_bytes.as_deref().map(parse_senary_compact_obj_animations).transpose()?;
    let indexed_graphics=indexed_graphics_bytes.as_deref().map(parse_indexed_graphics).transpose()?;
    let world_records38=world_records38_bytes.as_deref().map(parse_world_records38).transpose()?;
    let parameter_records27=parameter_records27_bytes.as_deref().map(parse_parameter_records27).transpose()?;
    let event_records12=event_records12_bytes.as_deref().map(parse_event_records12).transpose()?;
    let selection_byte_tables=selection_byte_tables_bytes.as_deref().map(parse_selection_byte_tables).transpose()?;
    let mission_records70=mission_records70_bytes.as_deref().map(parse_mission_records70).transpose()?;
    let conditional_records8=conditional_records8_bytes.as_deref().map(parse_conditional_records8).transpose()?;
    let records4=records4_bytes.as_deref().map(parse_records4).transpose()?;
    let metadata28=metadata28_bytes.as_deref().map(parse_metadata28).transpose()?;
    let encounter_coordinates=encounter_coordinates_bytes.as_deref().map(parse_encounter_coordinates).transpose()?;
    let encounter_unit_templates=encounter_unit_templates_bytes.as_deref().map(parse_encounter_unit_templates).transpose()?;
    let encounter_headers=encounter_headers_bytes.as_deref().map(parse_encounter_headers).transpose()?;
    let world_roads=world_roads_bytes.as_deref().map(parse_world_roads).transpose()?;
    let world_nodes=world_nodes_bytes.as_deref().map(parse_world_nodes).transpose()?;
    let word_task_obj_animations=word_task_obj_animations_bytes.as_deref().map(parse_word_task_obj_animations).transpose()?;
    let map_raster_palettes=map_raster_palettes_bytes.as_deref().map(parse_map_raster_palettes).transpose()?;
    let auxiliary_commands=auxiliary_commands_bytes.as_deref().map(parse_auxiliary_commands).transpose()?;
    let auxiliary_archive=auxiliary_archive_bytes.as_deref().map(parse_auxiliary_archive).transpose()?;
    Ok(Dataset {
        manifest,
        dimensions,
        obj_bytes,
        background,
        bg_bytes,
        palettes,
        palette_bytes,
        alternate_palettes,
        alternate_palette_bytes,
        packet_programs,
        packet_program_bytes,
        main_graphics,
        main_graphic_bytes,
        transfers,
        transfer_bytes,
        short_texts,
        short_text_bytes,
        font_bank,
        font_bank_bytes,
        name_keyboard,
        name_keyboard_bytes,
        obj_lists,
        obj_list_bytes,
        obj_animations,
        obj_animation_bytes,
        obj_graphics,
        obj_graphic_bytes,
        ui_palettes,
        ui_palette_bytes,
        ui_graphics,
        ui_graphic_bytes,
        items,
        item_bytes,
        item_links,
        item_link_bytes,
        metadata16,
        metadata52,
        metadata4,
        metadata2,
        default48,
        constructor_alternates,
        metadata14,
        metadata12,
        global_pair_flags,
        evaluation_parameters,
        progression_rewards,
        progression_gates,
        progression_selection_gates,
        aggregate_flag_keys,
        trade_parameters,
        item_effect_parameters,
        audio_directory,
        audio_tones,
        audio_sequences,
        audio_driver,
        palette_effect_dispatch,
        palette_programs,
        blend_programs,
        task_descriptors,
        effect_event_directories,
        script_commands,
        script_archive,
        script_text,
        script_coordinates,
        ui_choice_lists,
        unit_obj_initial_pose,
        color_mode_palettes,
        unit_obj_equipment,
        unit_obj_normal_battle,
        unit_obj_alternate_battle,
        unit_obj_script_special_normal,
        unit_obj_script_special_alternate,
        unit_obj_script_override,
        unit_obj_script_registry_special,
        unit_obj_script_registry_generic,
        unit_obj_property_words,
        unit_obj_byte_property_words,
        unit_obj_ffff_reader_fields,
        slot_job_lookup,
        observed_task_parameters,
        constructor_obj_animation,
        task_cycle_deltas,
        word_task_obj_animations,
        world_nodes,
        world_roads,
        encounter_headers,
        encounter_unit_templates,
        encounter_coordinates,
        metadata28,
        records4,
        conditional_records8,
        mission_records70,
        selection_byte_tables,
        event_records12,
        parameter_records27,
        world_records38,
        indexed_graphics,
        ui_raw_transfers,
        dynamic_raw_fragments,
        conditional_four_byte_lookup,
        conditional_packed_lookup,
        conditional_descriptor_banks,
        conditional_descriptor_bank_zero,
        conditional_property_masks,
        compact_obj_animations,
        secondary_compact_obj_animations,
        tertiary_compact_obj_animations,
        quaternary_compact_obj_animations,
        quinary_compact_obj_animations,
        senary_compact_obj_animations,
        map_raster_palettes,
        auxiliary_commands,
        auxiliary_archive,
        metadata16_bytes,
        metadata52_bytes,
        metadata4_bytes,
        metadata2_bytes,
        default48_bytes,
        constructor_alternates_bytes,
        metadata14_bytes,
        metadata12_bytes,
        global_pair_flags_bytes,
        evaluation_parameters_bytes,
        progression_rewards_bytes,
        progression_gates_bytes,
        progression_selection_gates_bytes,
        aggregate_flag_keys_bytes,
        trade_parameters_bytes,
        item_effect_parameters_bytes,
        audio_directory_bytes,
        audio_tones_bytes,
        audio_sequences_bytes,
        audio_driver_bytes,
        palette_effect_dispatch_bytes,
        palette_programs_bytes,
        blend_programs_bytes,
        task_descriptors_bytes,
        effect_event_directories_bytes,
        script_commands_bytes,
        script_archive_bytes,
        script_text_bytes,
        script_coordinates_bytes,
        ui_choice_lists_bytes,
        unit_obj_initial_pose_bytes,
        color_mode_palettes_bytes,
        unit_obj_equipment_bytes,
        unit_obj_normal_battle_bytes,
        unit_obj_alternate_battle_bytes,
        unit_obj_script_special_normal_bytes,
        unit_obj_script_special_alternate_bytes,
        unit_obj_script_override_bytes,
        unit_obj_script_registry_special_bytes,
        unit_obj_script_registry_generic_bytes,
        unit_obj_property_words_bytes,
        unit_obj_byte_property_words_bytes,
        unit_obj_ffff_reader_fields_bytes,
        slot_job_lookup_bytes,
        observed_task_parameters_bytes,
        constructor_obj_animation_bytes,
        task_cycle_deltas_bytes,
        word_task_obj_animations_bytes,
        world_nodes_bytes,
        world_roads_bytes,
        encounter_headers_bytes,
        encounter_unit_templates_bytes,
        encounter_coordinates_bytes,
        metadata28_bytes,
        records4_bytes,
        conditional_records8_bytes,
        mission_records70_bytes,
        selection_byte_tables_bytes,
        event_records12_bytes,
        parameter_records27_bytes,
        world_records38_bytes,
        indexed_graphics_bytes,
        ui_raw_transfers_bytes,
        dynamic_raw_fragments_bytes,
        conditional_four_byte_lookup_bytes,
        conditional_packed_lookup_bytes,
        conditional_descriptor_banks_bytes,
        conditional_descriptor_bank_zero_bytes,
        conditional_property_masks_bytes,
        compact_obj_animations_bytes,
        secondary_compact_obj_animations_bytes,
        tertiary_compact_obj_animations_bytes,
        quaternary_compact_obj_animations_bytes,
        quinary_compact_obj_animations_bytes,
        senary_compact_obj_animations_bytes,
        map_raster_palettes_bytes,
        auxiliary_commands_bytes,
        auxiliary_archive_bytes,
        text_catalogs,
        text_catalog_bytes,
    })
}

fn check_dataset_budget(
    manifest: usize,
    obj: usize,
    bg: usize,
    palettes: usize,
    alternate_palettes: usize,
    packet_programs: usize,
    main_graphics: usize,
    transfers: usize,
    short_texts: usize,
    text_catalogs: usize,
    font_bank: usize,
    name_keyboard: usize,
    obj_lists: usize,
    obj_animations: usize,
    obj_graphics: usize,
    ui_palettes: usize,
    ui_graphics: usize,
    items: usize,
    item_links: usize,
    metadata16: usize,
    metadata52: usize,
    metadata4: usize,
    metadata2: usize,
    default48: usize,
    constructor_alternates: usize,
    metadata14: usize,
    metadata12: usize,
    global_pair_flags: usize,
    evaluation_parameters: usize,
    progression_rewards: usize,
    progression_gates: usize,
    progression_selection_gates: usize,
    aggregate_flag_keys: usize,
    trade_parameters: usize,
    item_effect_parameters: usize,
    audio_directory: usize,
    audio_tones: usize,
    audio_sequences: usize,
    audio_driver: usize,
    palette_effect_dispatch: usize,
    palette_programs: usize,
    blend_programs: usize,
    task_descriptors: usize,
    effect_event_directories: usize,
    script_commands: usize,
    script_archive: usize,
    script_text: usize,
    script_coordinates: usize,
    ui_choice_lists: usize,
    unit_obj_initial_pose: usize,
    color_mode_palettes: usize,
    unit_obj_equipment: usize,
    unit_obj_normal_battle: usize,
    unit_obj_alternate_battle: usize,
    unit_obj_script_special_normal: usize,
    unit_obj_script_special_alternate: usize,
    unit_obj_script_override: usize,
    unit_obj_script_registry_special: usize,
    unit_obj_script_registry_generic: usize,
    unit_obj_property_words: usize,
    unit_obj_byte_property_words: usize,
    unit_obj_ffff_reader_fields: usize,
    slot_job_lookup: usize,
    observed_task_parameters: usize,
    constructor_obj_animation: usize,
    task_cycle_deltas: usize,
    word_task_obj_animations: usize,
    world_nodes: usize,
    world_roads: usize,
    encounter_headers: usize,
    encounter_unit_templates: usize,
    encounter_coordinates: usize,
    metadata28: usize,
    records4: usize,
    conditional_records8: usize,
    mission_records70: usize,
    selection_byte_tables: usize,
    event_records12: usize,
    parameter_records27: usize,
    world_records38: usize,
    indexed_graphics: usize,
    ui_raw_transfers: usize,
    dynamic_raw_fragments: usize,
    conditional_four_byte_lookup: usize,
    conditional_packed_lookup: usize,
    conditional_descriptor_banks: usize,
    conditional_descriptor_bank_zero: usize,
    conditional_property_masks: usize,
    compact_obj_animations: usize,
    secondary_compact_obj_animations: usize,
    tertiary_compact_obj_animations: usize,
    quaternary_compact_obj_animations: usize,
    quinary_compact_obj_animations: usize,
    senary_compact_obj_animations: usize,
    map_raster_palettes: usize,
    auxiliary_commands: usize,
    auxiliary_archive: usize,
) -> Result<(), String> {
    if manifest as u64 > MAX_MANIFEST_FILE_BYTES
        || obj as u64 > MAX_FILE_BYTES
        || bg as u64 > MAX_BG_DESCRIPTORS_FILE_BYTES
        || palettes as u64 > MAX_BG_BASE_PALETTES_FILE_BYTES
        || alternate_palettes as u64 > MAX_BG_ALTERNATE_PALETTES_FILE_BYTES
        || packet_programs as u64 > MAX_BG_PACKET_PROGRAMS_FILE_BYTES
        || main_graphics as u64 > MAX_BG_MAIN_GRAPHICS_FILE_BYTES
        || transfers as u64 > MAX_BG_TRANSFER_PROGRAMS_FILE_BYTES
        || short_texts as u64 > MAX_SHORT_TEXTS_FILE_BYTES
        || font_bank as u64 > MAX_FONT_BANK_FILE_BYTES
        || name_keyboard as u64 > MAX_NAME_KEYBOARD_FILE_BYTES
        || obj_lists as u64 > MAX_OBJ_STATIC_LISTS_FILE_BYTES
        || obj_animations as u64 > MAX_OBJ_ANIMATION_BANKS_FILE_BYTES
        || obj_graphics as u64 > MAX_OBJ_CUSTOM_GRAPHICS_FILE_BYTES
        || ui_palettes as u64 > MAX_UI_RAW_PALETTES_FILE_BYTES
        || ui_graphics as u64 > MAX_UI_CUSTOM_GRAPHICS_FILE_BYTES
        || items as u64 > MAX_ITEM_RECORDS_FILE_BYTES
        || item_links as u64 > MAX_ITEM_LINKS_FILE_BYTES
        || metadata16 as u64 > MAX_METADATA16_FILE_BYTES
        || metadata52 as u64 > MAX_METADATA52_FILE_BYTES
        || metadata4 as u64 > MAX_METADATA4_FILE_BYTES
        || metadata2 as u64 > MAX_METADATA2_FILE_BYTES
        || default48 as u64 > MAX_DEFAULT48_FILE_BYTES
        || constructor_alternates as u64 > MAX_CONSTRUCTOR_ALTERNATES_FILE_BYTES
        || metadata14 as u64 > MAX_METADATA14_FILE_BYTES
        || metadata12 as u64 > MAX_METADATA12_FILE_BYTES
        || global_pair_flags as u64 > MAX_GLOBAL_PAIR_FLAGS_FILE_BYTES
        || evaluation_parameters as u64 > MAX_EVALUATION_PARAMETERS_FILE_BYTES
        || progression_rewards as u64 > MAX_PROGRESSION_REWARDS_FILE_BYTES
        || progression_gates as u64 > MAX_PROGRESSION_GATES_FILE_BYTES
        || progression_selection_gates as u64 > MAX_PROGRESSION_SELECTION_GATES_FILE_BYTES
        || aggregate_flag_keys as u64 > MAX_AGGREGATE_FLAG_KEYS_FILE_BYTES
        || trade_parameters as u64 > MAX_TRADE_PARAMETERS_FILE_BYTES
        || item_effect_parameters as u64 > MAX_ITEM_EFFECT_PARAMETERS_FILE_BYTES
        || audio_directory as u64 > MAX_AUDIO_DIRECTORY_FILE_BYTES
        || audio_tones as u64 > MAX_AUDIO_TONES_FILE_BYTES
        || audio_sequences as u64 > MAX_AUDIO_SEQUENCES_FILE_BYTES
        || audio_driver as u64 > MAX_AUDIO_DRIVER_FILE_BYTES
        || palette_effect_dispatch as u64 > MAX_PALETTE_EFFECT_DISPATCH_FILE_BYTES
        || palette_programs as u64 > MAX_PALETTE_PROGRAMS_FILE_BYTES
        || blend_programs as u64 > MAX_BLEND_PROGRAMS_FILE_BYTES
        || task_descriptors as u64 > MAX_TASK_DESCRIPTORS_FILE_BYTES
        || effect_event_directories as u64 > MAX_EFFECT_EVENT_DIRECTORIES_FILE_BYTES
        || script_commands as u64 > MAX_SCRIPT_COMMANDS_FILE_BYTES
        || script_archive as u64 > MAX_SCRIPT_ARCHIVE_FILE_BYTES
        || script_text as u64 > MAX_SCRIPT_TEXT_FILE_BYTES
        || script_coordinates as u64 > MAX_SCRIPT_COORDINATES_FILE_BYTES
        || ui_choice_lists as u64 > MAX_UI_CHOICE_LISTS_FILE_BYTES
        || unit_obj_initial_pose as u64 > MAX_UNIT_OBJ_INITIAL_POSE_FILE_BYTES
        || color_mode_palettes as u64 > MAX_COLOR_MODE_PALETTES_FILE_BYTES
        || unit_obj_equipment as u64 > MAX_UNIT_OBJ_EQUIPMENT_FILE_BYTES
        || unit_obj_normal_battle as u64 > MAX_UNIT_OBJ_NORMAL_BATTLE_FILE_BYTES
        || unit_obj_alternate_battle as u64 > MAX_UNIT_OBJ_ALTERNATE_BATTLE_FILE_BYTES
        || unit_obj_script_special_normal as u64 > MAX_UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FILE_BYTES
        || unit_obj_script_special_alternate as u64 > MAX_UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FILE_BYTES
        || unit_obj_script_override as u64 > MAX_UNIT_OBJ_SCRIPT_OVERRIDE_FILE_BYTES
        || unit_obj_script_registry_special as u64 > MAX_UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FILE_BYTES
        || unit_obj_script_registry_generic as u64 > MAX_UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FILE_BYTES
        || unit_obj_property_words as u64 > MAX_UNIT_OBJ_PROPERTY_WORDS_FILE_BYTES
        || unit_obj_byte_property_words as u64 > MAX_UNIT_OBJ_BYTE_PROPERTY_WORDS_FILE_BYTES
        || unit_obj_ffff_reader_fields as u64 > MAX_UNIT_OBJ_FFFF_READER_FIELDS_FILE_BYTES
        || slot_job_lookup as u64 > MAX_SLOT_JOB_LOOKUP_FILE_BYTES
        || observed_task_parameters as u64 > MAX_OBSERVED_TASK_PARAMETERS_FILE_BYTES
        || constructor_obj_animation as u64 > MAX_CONSTRUCTOR_OBJ_ANIMATION_FILE_BYTES
        || task_cycle_deltas as u64 > MAX_TASK_CYCLE_DELTAS_FILE_BYTES
        || word_task_obj_animations as u64 > MAX_WORD_TASK_OBJ_ANIMATIONS_FILE_BYTES
        || world_nodes as u64 > MAX_WORLD_NODES_FILE_BYTES
        || world_roads as u64 > MAX_WORLD_ROADS_FILE_BYTES
        || encounter_headers as u64 > MAX_ENCOUNTER_HEADERS_FILE_BYTES
        || encounter_unit_templates as u64 > MAX_ENCOUNTER_UNIT_TEMPLATES_FILE_BYTES
        || encounter_coordinates as u64 > MAX_ENCOUNTER_COORDINATES_FILE_BYTES
        || metadata28 as u64 > MAX_METADATA28_FILE_BYTES
        || records4 as u64 > MAX_RECORDS4_FILE_BYTES
        || conditional_records8 as u64 > MAX_CONDITIONAL_RECORDS8_FILE_BYTES
        || mission_records70 as u64 > MAX_MISSION_RECORDS70_FILE_BYTES
        || selection_byte_tables as u64 > MAX_SELECTION_BYTE_TABLES_FILE_BYTES
        || event_records12 as u64 > MAX_EVENT_RECORDS12_FILE_BYTES
        || parameter_records27 as u64 > MAX_PARAMETER_RECORDS27_FILE_BYTES
        || world_records38 as u64 > MAX_WORLD_RECORDS38_FILE_BYTES
        || indexed_graphics as u64 > MAX_INDEXED_GRAPHICS_FILE_BYTES
        || ui_raw_transfers as u64 > MAX_UI_RAW_TRANSFERS_FILE_BYTES
        || dynamic_raw_fragments as u64 > MAX_DYNAMIC_RAW_FRAGMENTS_FILE_BYTES
        || conditional_four_byte_lookup as u64 > MAX_CONDITIONAL_FOUR_BYTE_LOOKUP_FILE_BYTES
        || conditional_packed_lookup as u64 > MAX_CONDITIONAL_PACKED_LOOKUP_FILE_BYTES
        || conditional_descriptor_banks as u64 > MAX_CONDITIONAL_DESCRIPTOR_BANKS_FILE_BYTES
        || conditional_descriptor_bank_zero as u64 > MAX_CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE_BYTES
        || conditional_property_masks as u64 > MAX_CONDITIONAL_PROPERTY_MASKS_FILE_BYTES
        || compact_obj_animations as u64 > MAX_COMPACT_OBJ_ANIMATIONS_FILE_BYTES
        || secondary_compact_obj_animations as u64 > MAX_SECONDARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES
        || tertiary_compact_obj_animations as u64 > MAX_TERTIARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES
        || quaternary_compact_obj_animations as u64 > MAX_QUATERNARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES
        || quinary_compact_obj_animations as u64 > MAX_QUINARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES
        || senary_compact_obj_animations as u64 > MAX_SENARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES
        || map_raster_palettes as u64 > MAX_MAP_RASTER_PALETTES_FILE_BYTES
        || auxiliary_commands as u64 > MAX_AUXILIARY_COMMANDS_FILE_BYTES
        || auxiliary_archive as u64 > MAX_AUXILIARY_ARCHIVE_FILE_BYTES
        || text_catalogs as u64 > MAX_TEXT_CATALOGS_FILE_BYTES
    {
        return Err("Resource file exceeds its size budget".into());
    }
    if manifest
        .saturating_add(obj)
        .saturating_add(bg)
        .saturating_add(palettes)
        .saturating_add(alternate_palettes)
        .saturating_add(packet_programs)
        .saturating_add(main_graphics)
        .saturating_add(transfers)
        .saturating_add(short_texts)
        .saturating_add(font_bank)
        .saturating_add(name_keyboard)
        .saturating_add(obj_lists)
        .saturating_add(obj_animations)
        .saturating_add(obj_graphics)
        .saturating_add(ui_palettes)
        .saturating_add(ui_graphics)
        .saturating_add(items)
        .saturating_add(item_links)
        .saturating_add(metadata16)
        .saturating_add(metadata52)
        .saturating_add(metadata4)
        .saturating_add(metadata2)
        .saturating_add(default48)
        .saturating_add(constructor_alternates)
        .saturating_add(metadata14)
        .saturating_add(metadata12)
        .saturating_add(global_pair_flags)
        .saturating_add(evaluation_parameters)
        .saturating_add(progression_rewards)
        .saturating_add(progression_gates)
        .saturating_add(progression_selection_gates)
        .saturating_add(aggregate_flag_keys)
        .saturating_add(trade_parameters)
        .saturating_add(item_effect_parameters)
        .saturating_add(audio_directory)
        .saturating_add(audio_tones)
        .saturating_add(audio_sequences)
        .saturating_add(audio_driver)
        .saturating_add(palette_effect_dispatch)
        .saturating_add(palette_programs)
        .saturating_add(blend_programs)
        .saturating_add(task_descriptors)
        .saturating_add(effect_event_directories)
        .saturating_add(script_commands)
        .saturating_add(script_archive)
        .saturating_add(script_text)
        .saturating_add(script_coordinates)
        .saturating_add(ui_choice_lists)
        .saturating_add(unit_obj_initial_pose)
        .saturating_add(color_mode_palettes)
        .saturating_add(unit_obj_equipment)
        .saturating_add(unit_obj_normal_battle)
        .saturating_add(unit_obj_alternate_battle)
        .saturating_add(unit_obj_script_special_normal)
        .saturating_add(unit_obj_script_special_alternate)
        .saturating_add(unit_obj_script_override)
        .saturating_add(unit_obj_script_registry_special)
        .saturating_add(unit_obj_script_registry_generic)
        .saturating_add(unit_obj_property_words)
        .saturating_add(unit_obj_byte_property_words)
        .saturating_add(unit_obj_ffff_reader_fields)
        .saturating_add(slot_job_lookup)
        .saturating_add(observed_task_parameters)
        .saturating_add(constructor_obj_animation)
        .saturating_add(task_cycle_deltas)
        .saturating_add(word_task_obj_animations)
        .saturating_add(world_nodes)
        .saturating_add(world_roads)
        .saturating_add(encounter_headers)
        .saturating_add(encounter_unit_templates)
        .saturating_add(encounter_coordinates)
        .saturating_add(metadata28)
        .saturating_add(records4)
        .saturating_add(conditional_records8)
        .saturating_add(mission_records70)
        .saturating_add(selection_byte_tables)
        .saturating_add(event_records12)
        .saturating_add(parameter_records27)
        .saturating_add(world_records38)
        .saturating_add(indexed_graphics)
        .saturating_add(ui_raw_transfers)
        .saturating_add(dynamic_raw_fragments)
        .saturating_add(conditional_four_byte_lookup)
        .saturating_add(conditional_packed_lookup)
        .saturating_add(conditional_descriptor_banks)
        .saturating_add(conditional_descriptor_bank_zero)
        .saturating_add(conditional_property_masks)
        .saturating_add(compact_obj_animations)
        .saturating_add(secondary_compact_obj_animations)
        .saturating_add(tertiary_compact_obj_animations)
        .saturating_add(quaternary_compact_obj_animations)
        .saturating_add(quinary_compact_obj_animations)
        .saturating_add(senary_compact_obj_animations)
        .saturating_add(map_raster_palettes)
        .saturating_add(auxiliary_commands)
        .saturating_add(auxiliary_archive)
        .saturating_add(text_catalogs)
        > MAX_DATASET_BYTES
    {
        return Err("Resource dataset exceeds its total size budget".into());
    }
    Ok(())
}

pub fn attach_from_directory(machine: &mut gba_core::Machine, path: &Path) -> Result<(), String> {
    let dataset = read_dataset(path)?;
    let resources =
        gba_core::rom_data::RuntimeResources::from_base_files(&machine.bus.rom, dataset.files())?;
    let obj_resident = resources.obj_resident_bytes() + 2 * std::mem::size_of::<usize>();
    let bg_resident = resources.background_resident_bytes();
    let palette_resident = resources.palette_resident_bytes();
    let resident = resources.resident_bytes() + 2 * std::mem::size_of::<usize>();
    machine.bus.set_rom_resources(Some(resources.clone()))?;
    eprintln!("ROM_DATA runtime=decoded resource={} source={} dataset={} file={} resident_bytes={obj_resident} total_resident_bytes={resident}",
        resources.resource_id(), resources.source_sha256(), resources.dataset_sha256(), resources.file_sha256());
    eprintln!(
        "ROM_DATA runtime=decoded resource={BG_DESCRIPTORS_ID} source={} dataset={} file={} records={} resident_bytes={bg_resident}",
        resources.source_sha256(),resources.dataset_sha256(),
        resources.background_file_sha256().unwrap(),dataset.background.entries.len()
    );
    eprintln!("ROM_DATA runtime=decoded resource={BG_BASE_PALETTES_ID} source={} dataset={} file={} records={} resident_bytes={palette_resident}", resources.source_sha256(), resources.dataset_sha256(), resources.palette_file_sha256().unwrap(), dataset.palettes.entries.len());
    eprintln!("ROM_DATA runtime=decoded resource={BG_ALTERNATE_PALETTES_ID} source={} dataset={} file={} catalogs=2 keys=157", resources.source_sha256(), resources.dataset_sha256(), resources.alternate_palette_file_sha256().unwrap());
    let packet_mode = if resources.packet_file_sha256().is_some() { "decoded" } else { "verified-only" };
    eprintln!("ROM_DATA runtime={packet_mode} resource={BG_PACKET_PROGRAMS_ID} source={} dataset={} file={} programs={}",
        resources.source_sha256(), resources.dataset_sha256(), dataset.manifest.resources[4].sha256,
        dataset.packet_programs.paired.programs.len() + dataset.packet_programs.halfwords.programs.len() + dataset.packet_programs.auxiliary.programs.len());
    if let Some(graphics) = &dataset.main_graphics {
        eprintln!("ROM_DATA runtime=decoded resource={BG_MAIN_GRAPHICS_ID} source={} dataset={} file={} graphics={}",
            resources.source_sha256(), resources.dataset_sha256(), dataset.manifest.resources[5].sha256,
            graphics.graphics.len());
    }
    if let Some(data)= &dataset.metadata16 {
        eprintln!("ROM_DATA runtime=decoded resource={METADATA16_ID} file={} records={}",dataset.manifest.resources[18].sha256,data.records.len());
    }
    if let Some(data)= &dataset.metadata52 {
        eprintln!("ROM_DATA runtime=decoded resource={METADATA52_ID} file={} records={}",dataset.manifest.resources[19].sha256,data.records.len());
    }
    if let Some(data)= &dataset.metadata4 {
        eprintln!("ROM_DATA runtime=decoded resource={METADATA4_ID} file={} records={}",dataset.manifest.resources[20].sha256,data.records.len());
    }
    if let Some(data)= &dataset.metadata2 {
        eprintln!("ROM_DATA runtime=decoded resource={METADATA2_ID} file={} records={}",dataset.manifest.resources[21].sha256,data.records.len());
    }
    if let Some(_data)= &dataset.default48 {
        eprintln!("ROM_DATA runtime=decoded resource={DEFAULT48_ID} file={} templates={}",dataset.manifest.resources[22].sha256,1);
    }
    if let Some(data)= &dataset.constructor_alternates {
        eprintln!("ROM_DATA runtime=decoded resource={CONSTRUCTOR_ALTERNATES_ID} file={} records={}",dataset.manifest.resources[23].sha256,data.records.len());
    }
    if let Some(data)= &dataset.metadata14 {
        eprintln!("ROM_DATA runtime=decoded resource={METADATA14_ID} file={} records={}",dataset.manifest.resources[24].sha256,data.records.len());
    }
    if let Some(data)= &dataset.metadata12 {
        eprintln!("ROM_DATA runtime=decoded resource={METADATA12_ID} file={} records={}",dataset.manifest.resources[25].sha256,data.banks.iter().map(|b|b.records.len()).sum::<usize>());
    }
    if let Some(data)= &dataset.global_pair_flags {
        eprintln!("ROM_DATA runtime=decoded resource={GLOBAL_PAIR_FLAGS_ID} file={} pairs={}",dataset.manifest.resources[26].sha256,data.groups.len()*4);
    }
    if let Some(data)= &dataset.evaluation_parameters {
        eprintln!("ROM_DATA runtime=decoded resource={EVALUATION_PARAMETERS_ID} file={} terms={} bonuses={} thresholds={}",dataset.manifest.resources[27].sha256,data.terms.len(),data.bonuses.len(),data.thresholds.len());
    }
    if let Some(data)= &dataset.progression_rewards {
        eprintln!("ROM_DATA runtime=decoded resource={PROGRESSION_REWARDS_ID} file={} records={}",dataset.manifest.resources[28].sha256,data.records.len());
    }
    if let Some(data)= &dataset.progression_gates {
        eprintln!("ROM_DATA runtime=decoded resource={PROGRESSION_GATES_ID} file={} records={}",dataset.manifest.resources[29].sha256,data.records.len());
    }
    if let Some(data)= &dataset.progression_selection_gates {
        eprintln!("ROM_DATA runtime=decoded resource={PROGRESSION_SELECTION_GATES_ID} file={} records={}",dataset.manifest.resources[30].sha256,data.records.len());
    }
    if let Some(data)= &dataset.aggregate_flag_keys {
        eprintln!("ROM_DATA runtime=decoded resource={AGGREGATE_FLAG_KEYS_ID} file={} records={}",dataset.manifest.resources[31].sha256,data.entries.len());
    }
    if let Some(data)= &dataset.trade_parameters {
        eprintln!("ROM_DATA runtime=decoded resource={TRADE_PARAMETERS_ID} file={} records={}",dataset.manifest.resources[32].sha256,data.records.len());
    }
    if let Some(data)= &dataset.item_effect_parameters {
        eprintln!("ROM_DATA runtime=decoded resource={ITEM_EFFECT_PARAMETERS_ID} file={} records={}",dataset.manifest.resources[33].sha256,data.records.len());
    }
    if let Some(data)= &dataset.audio_directory {
        eprintln!("ROM_DATA runtime=decoded resource={AUDIO_DIRECTORY_ID} file={} records={}",dataset.manifest.resources[34].sha256,data.songs.len());
    }
    if let Some(data)= &dataset.audio_tones {
        eprintln!("ROM_DATA runtime=decoded resource={AUDIO_TONES_ID} file={} records={}",dataset.manifest.resources[35].sha256,data.tones.len());
    }
    if let Some(data)= &dataset.audio_sequences {
        eprintln!("ROM_DATA runtime=decoded resource={AUDIO_SEQUENCES_ID} file={} records={}",dataset.manifest.resources[36].sha256,data.instructions.len());
    }
    if let Some(data)= &dataset.audio_driver {
        eprintln!("ROM_DATA runtime=decoded resource={AUDIO_DRIVER_ID} file={} records={}",dataset.manifest.resources[37].sha256,(0..7).map(|id|data.table(id).unwrap().1).sum::<usize>());
    }
    if let Some(data)= &dataset.palette_effect_dispatch {
        eprintln!("ROM_DATA runtime=decoded resource={PALETTE_EFFECT_DISPATCH_ID} file={} records={}",dataset.manifest.resources[38].sha256,data.slots.len());
    }
    if let Some(data)= &dataset.palette_programs {
        eprintln!("ROM_DATA runtime=decoded resource={PALETTE_PROGRAMS_ID} file={} records={}",dataset.manifest.resources[39].sha256,data.programs.len());
    }
    if let Some(data)= &dataset.blend_programs {
        eprintln!("ROM_DATA runtime=decoded resource={BLEND_PROGRAMS_ID} file={} records={}",dataset.manifest.resources[40].sha256,data.programs.len());
    }
    if let Some(data)= &dataset.task_descriptors {
        eprintln!("ROM_DATA runtime=decoded resource={TASK_DESCRIPTORS_ID} file={} records={}",dataset.manifest.resources[41].sha256,data.tables().iter().map(|t|t.entries.len()).sum::<usize>());
    }
    if let Some(data)= &dataset.effect_event_directories {
        eprintln!("ROM_DATA runtime=decoded resource={EFFECT_EVENT_DIRECTORIES_ID} file={} records={}",dataset.manifest.resources[42].sha256,data.events.len()+data.selectors.len());
    }
    if let Some(data)= &dataset.script_commands {
        eprintln!("ROM_DATA runtime=decoded resource={SCRIPT_COMMANDS_ID} file={} records={}",dataset.manifest.resources[43].sha256,data.commands.len());
    }
    if let Some(data)= &dataset.script_archive {
        eprintln!("ROM_DATA runtime=decoded resource={SCRIPT_ARCHIVE_ID} file={} records={}",dataset.manifest.resources[44].sha256,data.programs.iter().map(|p|p.instructions.len()).sum::<usize>()+data.keys.len());
    }
    if let Some(data)= &dataset.script_text {
        eprintln!("ROM_DATA runtime=decoded resource={SCRIPT_TEXT_ID} file={} records={}",dataset.manifest.resources[45].sha256,data.languages.iter().flat_map(|l|&l.banks).map(|b|b.programs.len()).sum::<usize>());
    }
    if let Some(data)= &dataset.script_coordinates {
        eprintln!("ROM_DATA runtime=decoded resource={SCRIPT_COORDINATES_ID} file={} records={}",dataset.manifest.resources[46].sha256,data.paths.len());
    }
    if let Some(data)= &dataset.ui_choice_lists {
        eprintln!("ROM_DATA runtime=decoded resource={UI_CHOICE_LISTS_ID} file={} records={}",dataset.manifest.resources[50].sha256,data.lists.len());
    }
    if let Some(data)= &dataset.unit_obj_initial_pose {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_INITIAL_POSE_ID} file={} records={}",dataset.manifest.resources[51].sha256,data.bindings.len());
    }
    if let Some(data)= &dataset.color_mode_palettes {
        eprintln!("ROM_DATA runtime=decoded resource={COLOR_MODE_PALETTES_ID} file={} records={}",dataset.manifest.resources[52].sha256,data.spans.len());
    }
    if let Some(data)= &dataset.unit_obj_equipment {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_EQUIPMENT_ID} file={} records={}",dataset.manifest.resources[53].sha256,data.bindings.len());
    }
    if let Some(data)= &dataset.unit_obj_normal_battle {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_NORMAL_BATTLE_ID} file={} records={}",dataset.manifest.resources[54].sha256,data.bindings.len());
    }
    if let Some(data)= &dataset.unit_obj_alternate_battle {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_ALTERNATE_BATTLE_ID} file={} records={}",dataset.manifest.resources[55].sha256,data.bindings.len());
    }
    if let Some(data)= &dataset.unit_obj_script_special_normal {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_ID} file={} records={}",dataset.manifest.resources[56].sha256,data.bindings.len());
    }
    if let Some(data)= &dataset.unit_obj_script_special_alternate {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_ID} file={} records={}",dataset.manifest.resources[57].sha256,data.bindings.len());
    }
    if let Some(data)= &dataset.unit_obj_script_override {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_SCRIPT_OVERRIDE_ID} file={} records={}",dataset.manifest.resources[58].sha256,data.bindings.len());
    }
    if let Some(data)= &dataset.unit_obj_script_registry_special {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_ID} file={} records={}",dataset.manifest.resources[59].sha256,data.bindings.len());
    }
    if let Some(data)= &dataset.unit_obj_script_registry_generic {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_ID} file={} records={}",dataset.manifest.resources[60].sha256,data.bindings.len());
    }
    if let Some(data)= &dataset.unit_obj_property_words {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_PROPERTY_WORDS_ID} file={} records={}",dataset.manifest.resources[61].sha256,data.words.len());
    }
    if let Some(data)= &dataset.unit_obj_byte_property_words {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_BYTE_PROPERTY_WORDS_ID} file={} records={}",dataset.manifest.resources[62].sha256,data.words.len());
    }
    if let Some(data)= &dataset.unit_obj_ffff_reader_fields {
        eprintln!("ROM_DATA runtime=decoded resource={UNIT_OBJ_FFFF_READER_FIELDS_ID} file={} fields=2 producers={}",dataset.manifest.resources[63].sha256,data.producers.len());
    }
    if let Some(data)= &dataset.slot_job_lookup {
        eprintln!("ROM_DATA runtime=decoded resource={SLOT_JOB_LOOKUP_ID} file={} entries={}",dataset.manifest.resources[64].sha256,data.entries.len());
    }
    if let Some(data)= &dataset.observed_task_parameters {
        eprintln!("ROM_DATA runtime=decoded resource={OBSERVED_TASK_PARAMETERS_ID} file={} phase_rows={} correction_rows={}",dataset.manifest.resources[65].sha256,data.phase_thresholds.entries.len(),data.vertical_corrections.entries.len());
    }
    if let Some(data)= &dataset.constructor_obj_animation {
        eprintln!("ROM_DATA runtime=decoded resource={CONSTRUCTOR_OBJ_ANIMATION_ID} file={} phases={}",dataset.manifest.resources[66].sha256,data.frames.len());
    }
    if let Some(data)= &dataset.task_cycle_deltas {
        eprintln!("ROM_DATA runtime=decoded resource={TASK_CYCLE_DELTAS_ID} file={} phases={}",dataset.manifest.resources[67].sha256,data.entries.len());
    }
    if let Some(data)=&dataset.ui_raw_transfers {eprintln!("ROM_DATA runtime=decoded resource={UI_RAW_TRANSFERS_ID} file={} views={}",dataset.manifest.resources[83].sha256,data.banks.len());}
    if let Some(data)=&dataset.dynamic_raw_fragments {eprintln!("ROM_DATA runtime=decoded resource={DYNAMIC_RAW_FRAGMENTS_ID} file={} rows={} banks={}",dataset.manifest.resources[84].sha256,data.rows.len(),data.banks.len());}
    if let Some(data)=&dataset.conditional_four_byte_lookup {eprintln!("ROM_DATA runtime=decoded resource={CONDITIONAL_FOUR_BYTE_LOOKUP_ID} file={} rows={}",dataset.manifest.resources[85].sha256,data.records.len());}
    if let Some(data)=&dataset.conditional_packed_lookup {eprintln!("ROM_DATA runtime=decoded resource={CONDITIONAL_PACKED_LOOKUP_ID} file={} rows={}",dataset.manifest.resources[86].sha256,data.records.len());}
    if let Some(data)=&dataset.conditional_descriptor_banks {eprintln!("ROM_DATA runtime=decoded resource={CONDITIONAL_DESCRIPTOR_BANKS_ID} file={} rows={}",dataset.manifest.resources[87].sha256,data.banks.len());}
    if let Some(data)=&dataset.conditional_descriptor_bank_zero {eprintln!("ROM_DATA runtime=decoded resource={CONDITIONAL_DESCRIPTOR_BANK_ZERO_ID} file={} rows={}",dataset.manifest.resources[88].sha256,data.records.len());}
    if let Some(data)=&dataset.conditional_property_masks {eprintln!("ROM_DATA runtime=decoded resource={CONDITIONAL_PROPERTY_MASKS_ID} file={} rows={}",dataset.manifest.resources[89].sha256,data.words.len());}
    if let Some(data)=&dataset.compact_obj_animations {eprintln!("ROM_DATA runtime=decoded resource={COMPACT_OBJ_ANIMATIONS_ID} file={} lists={} sequences={}",dataset.manifest.resources[90].sha256,data.lists.len(),data.sequences.len());}
    if let Some(data)=&dataset.secondary_compact_obj_animations {eprintln!("ROM_DATA runtime=decoded resource={SECONDARY_COMPACT_OBJ_ANIMATIONS_ID} file={} lists={} sequences={}",dataset.manifest.resources[91].sha256,data.lists.len(),data.sequences.len());}
    if let Some(data)=&dataset.tertiary_compact_obj_animations {eprintln!("ROM_DATA runtime=decoded resource={TERTIARY_COMPACT_OBJ_ANIMATIONS_ID} file={} lists={} sequences={}",dataset.manifest.resources[92].sha256,data.lists.len(),data.sequences.len());}
    if let Some(data)=&dataset.quaternary_compact_obj_animations {eprintln!("ROM_DATA runtime=decoded resource={QUATERNARY_COMPACT_OBJ_ANIMATIONS_ID} file={} lists={} sequences={}",dataset.manifest.resources[93].sha256,data.lists.len(),data.sequences.len());}
    if let Some(data)=&dataset.quinary_compact_obj_animations {eprintln!("ROM_DATA runtime=decoded resource={QUINARY_COMPACT_OBJ_ANIMATIONS_ID} file={} lists={} sequences={}",dataset.manifest.resources[94].sha256,data.lists.len(),data.sequences.len());}
    if let Some(data)=&dataset.senary_compact_obj_animations {eprintln!("ROM_DATA runtime=decoded resource={SENARY_COMPACT_OBJ_ANIMATIONS_ID} file={} lists={} sequences={}",dataset.manifest.resources[95].sha256,data.lists.len(),data.sequences.len());}
    if let Some(data)=&dataset.indexed_graphics {eprintln!("ROM_DATA runtime=decoded resource={INDEXED_GRAPHICS_ID} file={} archives={}",dataset.manifest.resources[82].sha256,data.archives.len());}
    if let Some(data)=&dataset.world_records38 {eprintln!("ROM_DATA runtime=decoded resource={WORLD_RECORDS38_ID} file={} records={}",dataset.manifest.resources[81].sha256,data.records.len());}
    if let Some(data)=&dataset.parameter_records27 {eprintln!("ROM_DATA runtime=decoded resource={PARAMETER_RECORDS27_ID} file={} records={}",dataset.manifest.resources[80].sha256,data.records.len());}
    if let Some(data)=&dataset.event_records12 {eprintln!("ROM_DATA runtime=decoded resource={EVENT_RECORDS12_ID} file={} records={}",dataset.manifest.resources[79].sha256,data.records.len());}
    if let Some(data)=&dataset.selection_byte_tables {eprintln!("ROM_DATA runtime=decoded resource={SELECTION_BYTE_TABLES_ID} file={} thresholds={} weights={}",dataset.manifest.resources[78].sha256,data.thresholds.len(),data.weights.len());}
    if let Some(data)=&dataset.mission_records70 {eprintln!("ROM_DATA runtime=decoded resource={MISSION_RECORDS70_ID} file={} records={}",dataset.manifest.resources[77].sha256,data.records.len());}
    if let Some(data)=&dataset.conditional_records8 {eprintln!("ROM_DATA runtime=decoded resource={CONDITIONAL_RECORDS8_ID} file={} records={}",dataset.manifest.resources[76].sha256,data.records.len());}
    if let Some(data)=&dataset.records4 {eprintln!("ROM_DATA runtime=decoded resource={RECORDS4_ID} file={} records={}",dataset.manifest.resources[75].sha256,data.records.len());}
    if let Some(data)=&dataset.metadata28 {eprintln!("ROM_DATA runtime=decoded resource={METADATA28_ID} file={} records={}",dataset.manifest.resources[74].sha256,data.records.len());}
    if let Some(data)=&dataset.encounter_coordinates {eprintln!("ROM_DATA runtime=decoded resource={ENCOUNTER_COORDINATES_ID} file={} lists={}",dataset.manifest.resources[73].sha256,data.lists.len());}
    if let Some(data)= &dataset.encounter_unit_templates {
        eprintln!("ROM_DATA runtime=decoded resource={ENCOUNTER_UNIT_TEMPLATES_ID} file={} records={}",dataset.manifest.resources[72].sha256,data.records.len());
    }
    if let Some(data)= &dataset.encounter_headers {
        eprintln!("ROM_DATA runtime=decoded resource={ENCOUNTER_HEADERS_ID} file={} headers={}",dataset.manifest.resources[71].sha256,data.headers.len());
    }
    if let Some(data)= &dataset.world_roads {
        eprintln!("ROM_DATA runtime=decoded resource={WORLD_ROADS_ID} file={} paths={}",dataset.manifest.resources[70].sha256,data.paths.len());
    }
    if let Some(data)= &dataset.world_nodes {
        eprintln!("ROM_DATA runtime=decoded resource={WORLD_NODES_ID} file={} nodes={}",dataset.manifest.resources[69].sha256,data.nodes.len());
    }
    if let Some(data)= &dataset.word_task_obj_animations {
        eprintln!("ROM_DATA runtime=decoded resource={WORD_TASK_OBJ_ANIMATIONS_ID} file={} banks={} lists={} frames={}",dataset.manifest.resources[68].sha256,data.banks.len(),data.banks.iter().map(|b|b.lists.len()).sum::<usize>(),data.banks.iter().flat_map(|b|&b.sequences).map(|s|s.frames.len()).sum::<usize>());
    }
    if let Some(data)= &dataset.map_raster_palettes {
        eprintln!("ROM_DATA runtime=decoded resource={MAP_RASTER_PALETTES_ID} file={} records={}",dataset.manifest.resources[49].sha256,data.programs.len());
    }
    if let Some(data)= &dataset.auxiliary_commands {
        eprintln!("ROM_DATA runtime=decoded resource={AUXILIARY_COMMANDS_ID} file={} records={}",dataset.manifest.resources[47].sha256,data.commands.len());
    }
    if let Some(data)= &dataset.auxiliary_archive {
        eprintln!("ROM_DATA runtime=decoded resource={AUXILIARY_ARCHIVE_ID} file={} records={}",dataset.manifest.resources[48].sha256,data.banks.iter().flat_map(|b|&b.programs).map(|p|p.instructions.len()).sum::<usize>());
    }
    if let Some(links)= &dataset.item_links {
        eprintln!("ROM_DATA runtime=decoded resource={ITEM_LINKS_ID} file={} records={}",dataset.manifest.resources[17].sha256,links.records.len());
    }
    if let Some(items) = &dataset.items {
        eprintln!("ROM_DATA runtime=decoded resource={ITEM_RECORDS_ID} file={} records={}",dataset.manifest.resources[16].sha256,items.records.len());
    }
    if let Some(graphics) = &dataset.ui_graphics {
        eprintln!("ROM_DATA runtime=decoded resource={UI_CUSTOM_GRAPHICS_ID} file={} banks={}", dataset.manifest.resources[15].sha256, graphics.banks.len());
    }
    if let Some(palettes) = &dataset.ui_palettes {
        eprintln!("ROM_DATA runtime=decoded resource={UI_RAW_PALETTES_ID} file={} banks={}", dataset.manifest.resources[14].sha256, palettes.banks.len());
    }
    if let Some(graphics) = &dataset.obj_graphics {
        eprintln!("ROM_DATA runtime=decoded resource={OBJ_CUSTOM_GRAPHICS_ID} file={} banks={} selector_tables={}", dataset.manifest.resources[13].sha256, graphics.banks.len(), graphics.pointer_tables.len());
    }
    if let Some(banks) = &dataset.obj_animations {
        eprintln!("ROM_DATA runtime=decoded resource={OBJ_ANIMATION_BANKS_ID} file={} banks={}", dataset.manifest.resources[12].sha256, banks.banks.len());
    }
    if let Some(lists) = &dataset.obj_lists {
        eprintln!("ROM_DATA runtime=decoded resource={OBJ_STATIC_LISTS_ID} file={} lists={}", dataset.manifest.resources[11].sha256, lists.lists.len());
    }
    if let (Some(font), Some(keyboard)) = (&dataset.font_bank, &dataset.name_keyboard) {
        eprintln!("ROM_DATA runtime=decoded resource={FONT_BANK_ID} file={} glyphs={}", dataset.manifest.resources[9].sha256, font.glyphs.len());
        eprintln!("ROM_DATA runtime=decoded resource={NAME_KEYBOARD_ID} file={} pages={}", dataset.manifest.resources[10].sha256, keyboard.programs.len());
    }
    if let Some(texts) = &dataset.text_catalogs {
        eprintln!("ROM_DATA runtime=decoded resource={TEXT_CATALOGS_ID} source={} dataset={} file={} pointer_catalogs={} archive_catalogs={} glyphs={}",
            resources.source_sha256(), resources.dataset_sha256(), dataset.manifest.resources[8].sha256,
            texts.pointer_catalogs.len(), texts.archive_catalogs.len(), texts.glyphs.len());
    }
    if let Some(texts) = &dataset.short_texts {
        let mode = if resources.text_file_sha256().is_some() { "decoded" } else { "verified-only" };
        eprintln!("ROM_DATA runtime={mode} resource={SHORT_TEXTS_ID} source={} dataset={} file={} languages={} strings={} glyphs={}",
            resources.source_sha256(), resources.dataset_sha256(), dataset.manifest.resources[7].sha256,
            texts.languages.len(), texts.strings.len(), texts.glyphs.len());
    }
    if let Some(transfers) = &dataset.transfers {
        eprintln!("ROM_DATA runtime=decoded resource={BG_TRANSFER_PROGRAMS_ID} source={} dataset={} file={} controls={} tiles={}",
            resources.source_sha256(), resources.dataset_sha256(), dataset.manifest.resources[6].sha256,
            transfers.controls.len(), transfers.tiles.len());
    }
    Ok(())
}

pub fn prepare_and_attach(machine: &mut gba_core::Machine, status: bool) -> Result<(), String> {
    let prepared = prepare_startup(&machine.bus.rom, status)?;
    attach_from_directory(machine, &prepared.directory)
}

pub fn report_usage(machine: &gba_core::Machine) {
    if let Some(resources) = machine.bus.rom_resources() {
        let stats = resources.stats();
        eprintln!("ROM_DATA_USE resource={} dataset={} bus_reads={} host_rom_reads={} host_ram_reads={} dynamic_ram_reads={} validation_reads={} bus_source_mask={}",
            resources.resource_id(), resources.dataset_sha256(), stats.bus_reads, stats.host_rom_reads,
            stats.host_ram_reads, stats.dynamic_ram_reads, stats.validation_reads, stats.bus_source_mask);
        if let Some(bg) = resources.background_stats() {
            eprintln!("ROM_DATA_USE resource={BG_DESCRIPTORS_ID} dataset={} bus_reads={} projected_bytes={}",
                resources.dataset_sha256(),bg.bus_reads,bg.projected_bytes);
        }
        if let Some(palette) = resources.palette_stats() {
            eprintln!("ROM_DATA_USE resource={BG_BASE_PALETTES_ID} dataset={} bus_reads={} source_bytes={} lz77_calls={} decoded_bytes={}", resources.dataset_sha256(), palette.bus_reads, palette.source_bytes, palette.lz77_calls, palette.decoded_bytes);
        }
        if let Some(alternate) = resources.alternate_palette_stats() {
            eprintln!("ROM_DATA_USE resource={BG_ALTERNATE_PALETTES_ID} dataset={} bus_reads={} source_bytes={} table_bytes={} decoded_literal_bytes={}", resources.dataset_sha256(), alternate.bus_reads, alternate.source_bytes, alternate.table_bytes, alternate.decoded_literal_bytes);
        }
        if let Some(packet) = resources.packet_stats() {
            eprintln!("ROM_DATA_USE resource={BG_PACKET_PROGRAMS_ID} dataset={} bus_reads={} source_bytes={} typed_value_bytes={} lz77_calls={} decoded_body_bytes={} typed_hle_bytes={}", resources.dataset_sha256(), packet.bus_reads, packet.source_bytes, packet.typed_value_bytes, packet.lz77_calls, packet.decoded_body_bytes, packet.typed_hle_bytes);
        }
        if let Some(graphics) = resources.graphics_stats() {
            eprintln!("ROM_DATA_USE resource={BG_MAIN_GRAPHICS_ID} dataset={} bus_reads={} source_bytes={} decoded_literal_bytes={}",
                resources.dataset_sha256(), graphics.bus_reads, graphics.source_bytes, graphics.decoded_literal_bytes);
        }
        if let Some(i)=resources.metadata16_stats() {
            eprintln!("ROM_DATA_USE resource={METADATA16_ID} dataset={} bus_reads={} source_bytes={} alias_bytes={} display_bytes={} flag_bytes={} value_bytes={} opaque_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.alias_bytes,i.display_bytes,i.flag_bytes,i.value_bytes,i.opaque_bytes);
        }
        if let Some(i)=resources.metadata52_stats() {
            eprintln!("ROM_DATA_USE resource={METADATA52_ID} dataset={} bus_reads={} source_bytes={} header_bytes={} routing_bytes={} packed_bytes={} tail_bytes={} opaque_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.header_bytes,i.routing_bytes,i.packed_bytes,i.tail_bytes,i.opaque_bytes);
        }
        if let Some(i)=resources.metadata4_stats() {
            eprintln!("ROM_DATA_USE resource={METADATA4_ID} dataset={} bus_reads={} source_bytes={} selector_bytes={} value_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.selector_bytes,i.value_bytes);
        }
        if let Some(i)=resources.conditional_descriptor_bank_zero_stats() {
            eprintln!("ROM_DATA_USE resource={CONDITIONAL_DESCRIPTOR_BANK_ZERO_ID} dataset={} bus_reads={} source_bytes={} pointer_bytes={} field0_bytes={} field2_bytes={} field4_bytes={} kind_bytes={} byte7_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.pointer_bytes,i.field0_bytes,i.field2_bytes,i.field4_bytes,i.kind_bytes,i.byte7_bytes);
        }
        if let Some(i)=resources.conditional_property_masks_stats() {
            eprintln!("ROM_DATA_USE resource={CONDITIONAL_PROPERTY_MASKS_ID} dataset={} bus_reads={} source_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes);
        }
        if let Some(i)=resources.compact_obj_animations_stats() {
            eprintln!("ROM_DATA_USE resource={COMPACT_OBJ_ANIMATIONS_ID} dataset={} bus_reads={} source_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes);
        }
        if let Some(i)=resources.secondary_compact_obj_animations_stats() {
            eprintln!("ROM_DATA_USE resource={SECONDARY_COMPACT_OBJ_ANIMATIONS_ID} dataset={} bus_reads={} source_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes);
        }
        if let Some(i)=resources.tertiary_compact_obj_animations_stats() {
            eprintln!("ROM_DATA_USE resource={TERTIARY_COMPACT_OBJ_ANIMATIONS_ID} dataset={} bus_reads={} source_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes);
        }
        if let Some(i)=resources.quaternary_compact_obj_animations_stats() {
            eprintln!("ROM_DATA_USE resource={QUATERNARY_COMPACT_OBJ_ANIMATIONS_ID} dataset={} bus_reads={} source_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes);
        }
        if let Some(i)=resources.quinary_compact_obj_animations_stats() {
            eprintln!("ROM_DATA_USE resource={QUINARY_COMPACT_OBJ_ANIMATIONS_ID} dataset={} bus_reads={} source_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes);
        }
        if let Some(i)=resources.senary_compact_obj_animations_stats() {
            eprintln!("ROM_DATA_USE resource={SENARY_COMPACT_OBJ_ANIMATIONS_ID} dataset={} bus_reads={} source_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes);
        }
        if let Some(i)=resources.conditional_descriptor_banks_stats() {
            eprintln!("ROM_DATA_USE resource={CONDITIONAL_DESCRIPTOR_BANKS_ID} dataset={} bus_reads={} source_bytes={} pointer_bytes={} field0_bytes={} field2_bytes={} field4_bytes={} kind_bytes={} byte7_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.pointer_bytes,i.field0_bytes,i.field2_bytes,i.field4_bytes,i.kind_bytes,i.byte7_bytes);
        }
        if let Some(i)=resources.conditional_packed_lookup_stats() {
            eprintln!("ROM_DATA_USE resource={CONDITIONAL_PACKED_LOOKUP_ID} dataset={} bus_reads={} source_bytes={} byte0_bytes={} packed1_bytes={} packed2_bytes={} byte3_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.byte0_bytes,i.packed1_bytes,i.packed2_bytes,i.byte3_bytes);
        }
        if let Some(i)=resources.conditional_four_byte_lookup_stats() {
            eprintln!("ROM_DATA_USE resource={CONDITIONAL_FOUR_BYTE_LOOKUP_ID} dataset={} bus_reads={} source_bytes={} first_field_bytes={} second_field_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.first_field_bytes,i.second_field_bytes);
        }
        if let Some(i)=resources.metadata2_stats() {
            eprintln!("ROM_DATA_USE resource={METADATA2_ID} dataset={} bus_reads={} source_bytes={} key_bytes={} value_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.key_bytes,i.value_bytes);
        }
        if let Some(i)=resources.default48_stats() {
            eprintln!("ROM_DATA_USE resource={DEFAULT48_ID} dataset={} bus_reads={} source_bytes={} header_bytes={} argument_bytes={} halfword_bytes={} flag_bytes={} tail_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.header_bytes,i.argument_bytes,i.halfword_bytes,i.flag_bytes,i.tail_bytes);
        }
        if let Some(i)=resources.constructor_alternates_stats() {
            eprintln!("ROM_DATA_USE resource={CONSTRUCTOR_ALTERNATES_ID} dataset={} bus_reads={} source_bytes={} pointer_bytes={} record_mask={} header_bytes={} argument_bytes={} halfword_bytes={} flag_bytes={} tail_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.pointer_bytes,i.record_mask,i.header_bytes,i.argument_bytes,i.halfword_bytes,i.flag_bytes,i.tail_bytes);
        }
        if let Some(i)=resources.metadata14_stats() {
            eprintln!("ROM_DATA_USE resource={METADATA14_ID} dataset={} bus_reads={} source_bytes={} records_used={} byte_fields={} halfword_bytes={} packed_bytes={} tail_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.records_used(),i.byte_fields,i.halfword_bytes,i.packed_bytes,i.tail_bytes);
        }
        if let Some(i)=resources.metadata12_stats() {
            eprintln!("ROM_DATA_USE resource={METADATA12_ID} dataset={} bus_reads={} source_bytes={} records_used={} primary_bytes={} secondary_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.records_used(),i.primary_bytes,i.secondary_bytes);
        }
        if let Some(i)=resources.global_pair_flags_stats() {
            eprintln!("ROM_DATA_USE resource={GLOBAL_PAIR_FLAGS_ID} dataset={} bus_reads={} source_bytes={} bytes_used={} byte_mask={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.bytes_used(),i.byte_mask);
        }
        if let Some(i)=resources.evaluation_parameters_stats() {
            eprintln!("ROM_DATA_USE resource={EVALUATION_PARAMETERS_ID} dataset={} bus_reads={} source_bytes={} divisor_bytes={} weight_bytes={} bonus_bytes={} score_opaque_bytes={} threshold_bytes={} threshold_opaque_bytes={} threshold_mask={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.divisor_bytes,i.weight_bytes,i.bonus_bytes,i.score_opaque_bytes,i.threshold_bytes,i.threshold_opaque_bytes,i.threshold_mask);
        }
        if let Some(i)=resources.progression_rewards_stats() {
            eprintln!("ROM_DATA_USE resource={PROGRESSION_REWARDS_ID} dataset={} bus_reads={} source_bytes={} delta_bytes={} discount_bytes={} opaque_bytes={} record_mask_lo={} record_mask_hi={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.delta_bytes,i.discount_bytes,i.opaque_bytes,i.record_mask[0],i.record_mask[1]);
        }
        if let Some(i)=resources.progression_gates_stats() {
            eprintln!("ROM_DATA_USE resource={PROGRESSION_GATES_ID} dataset={} bus_reads={} source_bytes={} requirement_bytes={} return_bytes={} output_bytes={} unused_bytes={} record_mask={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.requirement_bytes,i.return_bytes,i.output_bytes,i.unused_bytes,i.record_mask);
        }
        if let Some(i)=resources.progression_selection_gates_stats() {
            eprintln!("ROM_DATA_USE resource={PROGRESSION_SELECTION_GATES_ID} dataset={} bus_reads={} source_bytes={} selector_bytes={} minimum01_bytes={} minimum02_bytes={} minimum04_bytes={} any_minima_bytes={} flag_bytes={} record_mask={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.selector_bytes,i.minimum01_bytes,i.minimum02_bytes,i.minimum04_bytes,i.any_minima_bytes,i.flag_bytes,i.record_mask);
        }
        if let Some(i)=resources.aggregate_flag_keys_stats() {
            eprintln!("ROM_DATA_USE resource={AGGREGATE_FLAG_KEYS_ID} dataset={} bus_reads={} source_bytes={} key_bytes={} terminator_bytes={} entry_mask={:?}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.key_bytes,i.terminator_bytes,i.entry_mask);
        }
        if let Some(i)=resources.trade_parameters_stats() {
            eprintln!("ROM_DATA_USE resource={TRADE_PARAMETERS_ID} dataset={} bus_reads={} source_bytes={} opaque_bytes={} markup_low_bytes={} shared41_bytes={} discount_bytes={} record_mask={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.opaque_bytes,i.markup_low_bytes,i.shared41_bytes,i.discount_bytes,i.record_mask);
        }
        if let Some(i)=resources.item_effect_parameters_stats() {
            eprintln!("ROM_DATA_USE resource={ITEM_EFFECT_PARAMETERS_ID} dataset={} bus_reads={} source_bytes={} coefficient_bytes={} shared05_bytes={} shared06_bytes={} record_mask={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.coefficient_bytes,i.shared05_bytes,i.shared06_bytes,i.record_mask);
        }
        if let Some(i)=resources.audio_directory_stats() {
            eprintln!("ROM_DATA_USE resource={AUDIO_DIRECTORY_ID} dataset={} bus_reads={} source_bytes={} player_bytes={} directory_bytes={} header_bytes={} track_pointer_bytes={} player_mask={} songs_read={} headers_read={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.player_bytes,i.directory_bytes,i.header_bytes,i.track_pointer_bytes,i.player_mask,i.song_masks.iter().map(|v|v.count_ones()).sum::<u32>(),i.header_masks.iter().map(|v|v.count_ones()).sum::<u32>());
        }
        if let Some(i)=resources.audio_tones_stats() {
            eprintln!("ROM_DATA_USE resource={AUDIO_TONES_ID} dataset={} bus_reads={} source_bytes={} tone_bytes={} keymap_bytes={} wave_header_bytes={} pcm_bytes={} endpoint_bytes={} cgb_bytes={} host_reads={} host_source_bytes={} host_wave_header_bytes={} host_pcm_bytes={} host_endpoint_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.tone_bytes,i.keymap_bytes,i.wave_header_bytes,i.pcm_bytes,i.endpoint_bytes,i.cgb_bytes,i.host_reads,i.host_source_bytes,i.host_wave_header_bytes,i.host_pcm_bytes,i.host_endpoint_bytes);
        }
        if let Some(i)=resources.audio_sequences_stats() {
            eprintln!("ROM_DATA_USE resource={AUDIO_SEQUENCES_ID} dataset={} bus_reads={} source_bytes={} opcode_bytes={} operand_bytes={} clock_bytes={} command_bytes={} extended_bytes={} instructions_used={} clock_mask={} command_mask={} extended_mask={} host_reads={} host_source_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.opcode_bytes,i.operand_bytes,i.clock_bytes,i.command_bytes,i.extended_bytes,i.instructions_used,i.clock_mask,i.command_mask,i.extended_mask,i.host_reads,i.host_source_bytes);
        }
        if let Some(i)=resources.audio_driver_stats() {
            eprintln!("ROM_DATA_USE resource={AUDIO_DRIVER_ID} dataset={} bus_reads={} source_bytes={} pcm_key_bytes={} pcm_base_bytes={} sample_count_bytes={} cgb_key_bytes={} cgb_base_bytes={} noise_bytes={} volume_bytes={} entries_used={:?} host_reads={} host_source_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.table_bytes[0],i.table_bytes[1],i.table_bytes[2],i.table_bytes[3],i.table_bytes[4],i.table_bytes[5],i.table_bytes[6],i.entries_used,i.host_reads,i.host_source_bytes);
        }
        if let Some(i)=resources.palette_effect_dispatch_stats() {
            eprintln!("ROM_DATA_USE resource={PALETTE_EFFECT_DISPATCH_ID} dataset={} bus_reads={} source_bytes={} callback_bytes={} null_bytes={} slot_mask={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.callback_bytes,i.null_bytes,i.slot_mask);
        }
        if let Some(i)=resources.palette_programs_stats() {
            eprintln!("ROM_DATA_USE resource={PALETTE_PROGRAMS_ID} dataset={} bus_reads={} source_bytes={} header_bytes={} command_bytes={} parameter_bytes={} producer_bytes={} programs_used={} producer_tables_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.header_bytes,i.command_bytes,i.parameter_bytes,i.producer_bytes,i.programs_used,i.producer_tables_used);
        }
        if let Some(i)=resources.blend_programs_stats() {
            eprintln!("ROM_DATA_USE resource={BLEND_PROGRAMS_ID} dataset={} bus_reads={} source_bytes={} header_bytes={} command_bytes={} producer_bytes={} programs_used={} producer_tables_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.header_bytes,i.command_bytes,i.producer_bytes,i.programs_used,i.producer_tables_used);
        }
        if let Some(i)=resources.task_descriptors_stats() {
            eprintln!("ROM_DATA_USE resource={TASK_DESCRIPTORS_ID} dataset={} bus_reads={} source_bytes={} callback_bytes={} context_size_bytes={} metadata_bytes={} records_used={} catalogs_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.callback_bytes,i.context_size_bytes,i.metadata_bytes,i.records_used,i.catalogs_used);
        }
        if let Some(i)=resources.effect_event_directories_stats() {
            eprintln!("ROM_DATA_USE resource={EFFECT_EVENT_DIRECTORIES_ID} dataset={} bus_reads={} source_bytes={} event_bytes={} selector_bytes={} dispatch_bytes={} event_records_used={} selectors_used={} dispatch_tables_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.event_bytes,i.selector_bytes,i.dispatch_bytes,i.event_records_used,i.selectors_used,i.dispatch_tables_used);
        }
        if let Some(i)=resources.script_commands_stats() {
            eprintln!("ROM_DATA_USE resource={SCRIPT_COMMANDS_ID} dataset={} bus_reads={} source_bytes={} advance_bytes={} callback_bytes={} commands_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.advance_bytes,i.callback_bytes,i.commands_used);
        }
        if let Some(i)=resources.script_archive_stats() {
            eprintln!("ROM_DATA_USE resource={SCRIPT_ARCHIVE_ID} dataset={} bus_reads={} source_bytes={} index_bytes={} opcode_bytes={} parameter_bytes={} key_bytes={} padding_bytes={} banks_used={} programs_used={} key_records_used={} padding_spans_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.index_bytes,i.opcode_bytes,i.parameter_bytes,i.key_bytes,i.padding_bytes,i.banks_used,i.programs_used,i.key_records_used,i.padding_spans_used);
        }
        if let Some(i)=resources.script_text_stats() {
            eprintln!("ROM_DATA_USE resource={SCRIPT_TEXT_ID} dataset={} bus_reads={} source_bytes={} top_index_bytes={} variant_index_bytes={} bank_index_bytes={} header_bytes={} encoded_control_bytes={} glyph_bytes={} command_bytes={} operand_bytes={} terminator_bytes={} decoded_tail_bytes={} alignment_bytes={} decoded_program_bytes={} languages_used={} banks_used={} programs_used={} variant_records_used={} alignment_spans_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.top_index_bytes,i.variant_index_bytes,i.bank_index_bytes,i.header_bytes,i.encoded_control_bytes,i.glyph_bytes,i.command_bytes,i.operand_bytes,i.terminator_bytes,i.decoded_tail_bytes,i.alignment_bytes,i.decoded_program_bytes,i.languages_used,i.banks_used,i.programs_used,i.variant_records_used,i.alignment_spans_used);
        }
        if let Some(i)=resources.script_coordinates_stats() {
            eprintln!("ROM_DATA_USE resource={SCRIPT_COORDINATES_ID} dataset={} bus_reads={} source_bytes={} index_bytes={} count_bytes={} coordinate_bytes={} alignment_bytes={} paths_used={} alignment_spans_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.index_bytes,i.count_bytes,i.coordinate_bytes,i.alignment_bytes,i.paths_used,i.alignment_spans_used);
        }
        if let Some(i)=resources.ui_choice_lists_stats() {
            eprintln!("ROM_DATA_USE resource={UI_CHOICE_LISTS_ID} dataset={} bus_reads={} source_bytes={} identifier_bytes={} coordinate_bytes={} opaque_bytes={} navigation_bytes={} lists_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.identifier_bytes,i.coordinate_bytes,i.opaque_bytes,i.navigation_bytes,i.lists_used);
        }
        if let Some(i)=resources.unit_obj_initial_pose_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_INITIAL_POSE_ID} dataset={} bus_reads={} source_bytes={} directory_bytes={} index_bytes={} sequence_header_bytes={} frame_bytes={} object_bytes={} pixel_bytes={} sequences_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.directory_bytes,i.index_bytes,i.sequence_header_bytes,i.frame_bytes,i.object_bytes,i.pixel_bytes,i.sequences_used);
        }
        if let Some(i)=resources.color_mode_palettes_stats() {
            eprintln!("ROM_DATA_USE resource={COLOR_MODE_PALETTES_ID} dataset={} bus_reads={} source_bytes={} unit_base_bytes={} companion_bytes={} supplemental_bytes={} shared_color_bytes={} special_unit_bytes={} selector_bytes={} fragments_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.unit_base_bytes,i.companion_bytes,i.supplemental_bytes,i.shared_color_bytes,i.special_unit_bytes,i.selector_bytes,i.fragments_used);
        }
        if let Some(i)=resources.unit_obj_equipment_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_EQUIPMENT_ID} dataset={} bus_reads={} source_bytes={} directory_bytes={} index_bytes={} sequence_header_bytes={} frame_bytes={} object_bytes={} pixel_bytes={} sequences_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.directory_bytes,i.index_bytes,i.sequence_header_bytes,i.frame_bytes,i.object_bytes,i.pixel_bytes,i.sequences_used);
        }
        if let Some(i)=resources.unit_obj_normal_battle_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_NORMAL_BATTLE_ID} dataset={} bus_reads={} source_bytes={} directory_bytes={} index_bytes={} sequence_header_bytes={} frame_bytes={} object_bytes={} pixel_bytes={} sequences_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.directory_bytes,i.index_bytes,i.sequence_header_bytes,i.frame_bytes,i.object_bytes,i.pixel_bytes,i.sequences_used);
        }
        if let Some(i)=resources.unit_obj_alternate_battle_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_ALTERNATE_BATTLE_ID} dataset={} bus_reads={} source_bytes={} directory_bytes={} index_bytes={} sequence_header_bytes={} frame_bytes={} object_bytes={} pixel_bytes={} sequences_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.directory_bytes,i.index_bytes,i.sequence_header_bytes,i.frame_bytes,i.object_bytes,i.pixel_bytes,i.sequences_used);
        }
        if let Some(i)=resources.unit_obj_script_special_normal_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_ID} dataset={} bus_reads={} source_bytes={} directory_bytes={} index_bytes={} sequence_header_bytes={} frame_bytes={} object_bytes={} pixel_bytes={} sequences_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.directory_bytes,i.index_bytes,i.sequence_header_bytes,i.frame_bytes,i.object_bytes,i.pixel_bytes,i.sequences_used);
        }
        if let Some(i)=resources.unit_obj_script_special_alternate_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_ID} dataset={} bus_reads={} source_bytes={} directory_bytes={} index_bytes={} sequence_header_bytes={} frame_bytes={} object_bytes={} pixel_bytes={} sequences_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.directory_bytes,i.index_bytes,i.sequence_header_bytes,i.frame_bytes,i.object_bytes,i.pixel_bytes,i.sequences_used);
        }
        if let Some(i)=resources.unit_obj_script_override_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_SCRIPT_OVERRIDE_ID} dataset={} bus_reads={} source_bytes={} directory_bytes={} index_bytes={} sequence_header_bytes={} frame_bytes={} object_bytes={} pixel_bytes={} sequences_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.directory_bytes,i.index_bytes,i.sequence_header_bytes,i.frame_bytes,i.object_bytes,i.pixel_bytes,i.sequences_used);
        }
        if let Some(i)=resources.unit_obj_script_registry_special_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_ID} dataset={} bus_reads={} source_bytes={} directory_bytes={} index_bytes={} sequence_header_bytes={} frame_bytes={} object_bytes={} pixel_bytes={} sequences_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.directory_bytes,i.index_bytes,i.sequence_header_bytes,i.frame_bytes,i.object_bytes,i.pixel_bytes,i.sequences_used);
        }
        if let Some(i)=resources.unit_obj_script_registry_generic_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_ID} dataset={} bus_reads={} source_bytes={} directory_bytes={} index_bytes={} sequence_header_bytes={} frame_bytes={} object_bytes={} pixel_bytes={} sequences_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.directory_bytes,i.index_bytes,i.sequence_header_bytes,i.frame_bytes,i.object_bytes,i.pixel_bytes,i.sequences_used);
        }
        if let Some(i)=resources.unit_obj_property_words_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_PROPERTY_WORDS_ID} dataset={} bus_reads={} source_bytes={} words_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.words_used);
        }
        if let Some(i)=resources.unit_obj_byte_property_words_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_BYTE_PROPERTY_WORDS_ID} dataset={} bus_reads={} source_bytes={} words_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.words_used);
        }
        if let Some(i)=resources.unit_obj_ffff_reader_fields_stats() {
            eprintln!("ROM_DATA_USE resource={UNIT_OBJ_FFFF_READER_FIELDS_ID} dataset={} bus_reads={} source_bytes={} property_bytes={} directory_bytes={} fields_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.property_bytes,i.directory_bytes,i.fields_used);
        }
        if let Some(i)=resources.slot_job_lookup_stats() {
            eprintln!("ROM_DATA_USE resource={SLOT_JOB_LOOKUP_ID} dataset={} bus_reads={} source_bytes={} slots_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.slots_used);
        }
        if let Some(i)=resources.observed_task_parameters_stats() {
            eprintln!("ROM_DATA_USE resource={OBSERVED_TASK_PARAMETERS_ID} dataset={} bus_reads={} source_bytes={} phase_threshold_bytes={} vertical_correction_bytes={} phases_used={} corrections_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.phase_threshold_bytes,i.vertical_correction_bytes,i.phases_used,i.corrections_used);
        }
        if let Some(i)=resources.constructor_obj_animation_stats() {
            eprintln!("ROM_DATA_USE resource={CONSTRUCTOR_OBJ_ANIMATION_ID} dataset={} bus_reads={} source_bytes={} threshold_bytes={} pointer_bytes={} phases_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.threshold_bytes,i.pointer_bytes,i.phases_used);
        }
        if let Some(i)=resources.task_cycle_deltas_stats() {
            eprintln!("ROM_DATA_USE resource={TASK_CYCLE_DELTAS_ID} dataset={} bus_reads={} source_bytes={} phases_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.phases_used);
        }
        if let Some(i)=resources.ui_raw_transfers_stats() {eprintln!("ROM_DATA_USE resource={UI_RAW_TRANSFERS_ID} dataset={} bus_reads={} source_bytes={} selector_bytes={} payload_bytes={} banks_used={} groups_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.selector_bytes,i.payload_bytes,i.banks_used,i.groups_used);}
        if let Some(i)=resources.dynamic_raw_fragments_stats() {eprintln!("ROM_DATA_USE resource={DYNAMIC_RAW_FRAGMENTS_ID} dataset={} bus_reads={} source_bytes={} selector_bytes={} payload_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.selector_bytes,i.payload_bytes);}
        if let Some(i)=resources.indexed_graphics_stats() {eprintln!("ROM_DATA_USE resource={INDEXED_GRAPHICS_ID} dataset={} bus_reads={} source_bytes={} selector_bytes={} archive_header_bytes={} index_bytes={} token_bytes={} literal_bytes={} dictionary_header_bytes={} dictionary_bytes={} archives_used={} programs_used={} groups_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.selector_bytes,i.archive_header_bytes,i.index_bytes,i.token_bytes,i.literal_bytes,i.dictionary_header_bytes,i.dictionary_bytes,i.archives_used,i.programs_used,i.groups_used);}
        if let Some(i)=resources.world_records38_stats() {eprintln!("ROM_DATA_USE resource={WORLD_RECORDS38_ID} dataset={} bus_reads={} source_bytes={} byte01_bytes={} value02_bytes={} value04_bytes={} threshold_bytes={} opaque_bytes={} records_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.byte01_bytes,i.value02_bytes,i.value04_bytes,i.threshold_bytes,i.opaque_bytes,i.records_used);}
        if let Some(i)=resources.parameter_records27_stats() {eprintln!("ROM_DATA_USE resource={PARAMETER_RECORDS27_ID} dataset={} bus_reads={} source_bytes={} header_bytes={} packed_bytes={} value07_bytes={} byte09_bytes={} value0a_bytes={} signed0c_bytes={} flags0d_bytes={} operands0e_bytes={} operands10_bytes={} value12_bytes={} signed15_bytes={} opaque_bytes={} records_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.header_bytes,i.packed_bytes,i.value07_bytes,i.byte09_bytes,i.value0a_bytes,i.signed0c_bytes,i.flags0d_bytes,i.operands0e_bytes,i.operands10_bytes,i.value12_bytes,i.signed15_bytes,i.opaque_bytes,i.records_used);}
        if let Some(i)=resources.event_records12_stats() {eprintln!("ROM_DATA_USE resource={EVENT_RECORDS12_ID} dataset={} bus_reads={} source_bytes={} header_bytes={} key_bytes={} primary_bytes={} alternate_bytes={} parameter_bytes={} byte07_bytes={} flag_bytes={} opaque_bytes={} records_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.header_bytes,i.key_bytes,i.primary_bytes,i.alternate_bytes,i.parameter_bytes,i.byte07_bytes,i.flag_bytes,i.opaque_bytes,i.records_used);}
        if let Some(i)=resources.selection_byte_tables_stats() {eprintln!("ROM_DATA_USE resource={SELECTION_BYTE_TABLES_ID} dataset={} bus_reads={} source_bytes={} threshold_bytes={} weight_bytes={} threshold_positions_used={} weight_positions_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.threshold_bytes,i.weight_bytes,i.threshold_positions_used,i.weight_positions_used);}
        if let Some(i)=resources.mission_records70_stats() {eprintln!("ROM_DATA_USE resource={MISSION_RECORDS70_ID} dataset={} bus_reads={} source_bytes={} word_bytes={} packed_bytes={} operand_bytes={} inline_bytes={} byte_list_bytes={} scaled_bytes={} scalar_bytes={} mapped_bytes={} flag_bytes={} event_bytes={} records_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.word_bytes,i.packed_bytes,i.operand_bytes,i.inline_bytes,i.byte_list_bytes,i.scaled_bytes,i.scalar_bytes,i.mapped_bytes,i.flag_bytes,i.event_bytes,i.records_used);}
        if let Some(i)=resources.conditional_records8_stats() {eprintln!("ROM_DATA_USE resource={CONDITIONAL_RECORDS8_ID} dataset={} bus_reads={} source_bytes={} gate_bytes={} filter_bytes={} first_key_bytes={} first_operand_bytes={} second_key_bytes={} second_operand_bytes={} records_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.gate_bytes,i.filter_bytes,i.first_key_bytes,i.first_operand_bytes,i.second_key_bytes,i.second_operand_bytes,i.records_used);}
        if let Some(i)=resources.records4_stats() {eprintln!("ROM_DATA_USE resource={RECORDS4_ID} dataset={} bus_reads={} source_bytes={} field00_bytes={} field01_bytes={} field02_bytes={} field03_bytes={} records_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.field00_bytes,i.field01_bytes,i.field02_bytes,i.field03_bytes,i.records_used);}
        if let Some(i)=resources.metadata28_stats() {eprintln!("ROM_DATA_USE resource={METADATA28_ID} dataset={} bus_reads={} source_bytes={} halfword_bytes={} byte_bytes={} inline_bytes={} flag_bytes={} opaque_bytes={} records_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.halfword_bytes,i.byte_bytes,i.inline_bytes,i.flag_bytes,i.opaque_bytes,i.records_used);}
        if let Some(i)=resources.encounter_coordinates_stats() {eprintln!("ROM_DATA_USE resource={ENCOUNTER_COORDINATES_ID} dataset={} bus_reads={} source_bytes={} placement0_bytes={} random_bytes={} placement2_bytes={} default_bytes={} x_bytes={} y_bytes={} lists_used={} headers_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.placement0_bytes,i.random_bytes,i.placement2_bytes,i.default_bytes,i.x_bytes,i.y_bytes,i.lists_used,i.headers_used);}
        if let Some(i)=resources.encounter_unit_templates_stats() {
            eprintln!("ROM_DATA_USE resource={ENCOUNTER_UNIT_TEMPLATES_ID} dataset={} bus_reads={} source_bytes={} selector_bytes={} argument_bytes={} equipment_bytes={} auxiliary_bytes={} ability_bit_bytes={} optional_bytes={} flag_bytes={} control_bytes={} opaque_bytes={} records_used={} headers_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.selector_bytes,i.argument_bytes,i.equipment_bytes,i.auxiliary_bytes,i.ability_bit_bytes,i.optional_bytes,i.flag_bytes,i.control_bytes,i.opaque_bytes,i.records_used,i.headers_used);
        }
        if let Some(i)=resources.encounter_headers_stats() {
            eprintln!("ROM_DATA_USE resource={ENCOUNTER_HEADERS_ID} dataset={} bus_reads={} source_bytes={} unit_count_bytes={} map_bytes={} template_pointer_bytes={} placement_parameter_bytes={} placement_count_bytes={} position_pointer_bytes={} control_bytes={} opaque_bytes={} headers_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.unit_count_bytes,i.map_bytes,i.template_pointer_bytes,i.placement_parameter_bytes,i.placement_count_bytes,i.position_pointer_bytes,i.control_bytes,i.opaque_bytes,i.headers_used);
        }
        if let Some(i)=resources.world_roads_stats() {
            eprintln!("ROM_DATA_USE resource={WORLD_ROADS_ID} dataset={} bus_reads={} source_bytes={} count_bytes={} pointer_bytes={} sample_bytes={} opaque_bytes={} paths_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.count_bytes,i.pointer_bytes,i.sample_bytes,i.opaque_bytes,i.paths_used);
        }
        if let Some(i)=resources.world_nodes_stats() {
            eprintln!("ROM_DATA_USE resource={WORLD_NODES_ID} dataset={} bus_reads={} source_bytes={} coordinate_bytes={} path_id_bytes={} neighbor_bytes={} opaque_bytes={} nodes_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.coordinate_bytes,i.path_id_bytes,i.neighbor_bytes,i.opaque_bytes,i.nodes_used);
        }
        if let Some(i)=resources.word_task_obj_stats() {
            eprintln!("ROM_DATA_USE resource={WORD_TASK_OBJ_ANIMATIONS_ID} dataset={} bus_reads={} source_bytes={} index_bytes={} count_bytes={} attribute_bytes={} sequence_header_bytes={} frame_list_bytes={} duration_bytes={} host_reads={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.index_bytes,i.count_bytes,i.attribute_bytes,i.sequence_header_bytes,i.frame_list_bytes,i.duration_bytes,i.host_reads);
        }
        if let Some(i)=resources.map_raster_palettes_stats() {
            eprintln!("ROM_DATA_USE resource={MAP_RASTER_PALETTES_ID} dataset={} bus_reads={} source_bytes={} index_bytes={} header_bytes={} color_bytes={} programs_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.index_bytes,i.header_bytes,i.color_bytes,i.programs_used);
        }
        if let Some(i)=resources.auxiliary_commands_stats() {
            eprintln!("ROM_DATA_USE resource={AUXILIARY_COMMANDS_ID} dataset={} bus_reads={} source_bytes={} advance_bytes={} callback_bytes={} commands_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.advance_bytes,i.callback_bytes,i.commands_used);
        }
        if let Some(i)=resources.auxiliary_archive_stats() {
            eprintln!("ROM_DATA_USE resource={AUXILIARY_ARCHIVE_ID} dataset={} bus_reads={} source_bytes={} index_bytes={} opcode_bytes={} parameter_bytes={} alignment_bytes={} banks_used={} programs_used={} alignment_spans_used={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.index_bytes,i.opcode_bytes,i.parameter_bytes,i.alignment_bytes,i.banks_used,i.programs_used,i.alignment_spans_used);
        }
        if let Some(i)=resources.item_link_stats() {
            eprintln!("ROM_DATA_USE resource={ITEM_LINKS_ID} dataset={} bus_reads={} source_bytes={} count_bytes={} selector_bytes={} value_bytes={} opaque_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.count_bytes,i.selector_bytes,i.value_bytes,i.opaque_bytes);
        }
        if let Some(i)=resources.item_stats() {
            eprintln!("ROM_DATA_USE resource={ITEM_RECORDS_ID} dataset={} bus_reads={} source_bytes={} halfword_bytes={} signed_bytes={} other_bytes={}",resources.dataset_sha256(),i.bus_reads,i.source_bytes,i.halfword_bytes,i.signed_bytes,i.other_bytes);
        }
        if let Some(g) = resources.ui_graphics_stats() {
            eprintln!("ROM_DATA_USE resource={UI_CUSTOM_GRAPHICS_ID} dataset={} bus_reads={} source_bytes={} decoded_literal_bytes={} selector_bytes={} localized_source_bytes={} localized_literal_bytes={}", resources.dataset_sha256(), g.bus_reads, g.source_bytes, g.decoded_literal_bytes,g.selector_bytes,g.localized_source_bytes,g.localized_literal_bytes);
        }
        if let Some(g) = resources.table_graphics_stats() {
            eprintln!("ROM_DATA_UI_TABLE_USE dataset={} bus_reads={} source_bytes={} decoded_literal_bytes={} selector_bytes={} banks_used={} groups_used={}", resources.dataset_sha256(), g.bus_reads,g.source_bytes,g.decoded_literal_bytes,g.selector_bytes,g.banks_used,g.groups_used);
        }
        for (offset, bytes) in resources.ui_graphics_used_sources() {
            eprintln!("ROM_DATA_UI_GRAPHICS_SOURCE source={offset:08x} source_bytes={bytes}");
        }
        if let Some(p) = resources.ui_palette_stats() {
            eprintln!("ROM_DATA_USE resource={UI_RAW_PALETTES_ID} dataset={} bus_reads={} source_bytes={}", resources.dataset_sha256(), p.bus_reads, p.source_bytes);
        }
        if let Some(g) = resources.obj_graphics_stats() {
            eprintln!("ROM_DATA_USE resource={OBJ_CUSTOM_GRAPHICS_ID} dataset={} bus_reads={} source_bytes={} decoded_literal_bytes={} selector_bytes={}", resources.dataset_sha256(), g.bus_reads, g.source_bytes, g.decoded_literal_bytes, g.selector_bytes);
        }
        if let Some(a) = resources.obj_animation_stats() {
            eprintln!("ROM_DATA_USE resource={OBJ_ANIMATION_BANKS_ID} dataset={} bus_reads={} source_bytes={} index_bytes={} count_bytes={} attribute_bytes={} sequence_header_bytes={} frame_list_bytes={} duration_bytes={} host_reads={} zero_count_selector_bytes={} zero_count_host_reads={} localized_selector_bytes={} localized_source_bytes={} localized_host_reads={}",
                resources.dataset_sha256(), a.bus_reads, a.source_bytes, a.index_bytes, a.count_bytes, a.attribute_bytes, a.sequence_header_bytes, a.frame_list_bytes, a.duration_bytes, a.host_reads, a.zero_count_selector_bytes, a.zero_count_host_reads, a.localized_selector_bytes, a.localized_source_bytes, a.localized_host_reads);
        }
        if let Some(l) = resources.obj_list_stats() {
            eprintln!("ROM_DATA_USE resource={OBJ_STATIC_LISTS_ID} dataset={} bus_reads={} source_bytes={} count_bytes={} attribute_bytes={} host_reads={}",
                resources.dataset_sha256(), l.bus_reads, l.source_bytes, l.count_bytes, l.attribute_bytes, l.host_reads);
        }
        for (offset, guest, host) in resources.obj_list_used_sources() {
            eprintln!("ROM_DATA_OBJ_SOURCE source={offset:08x} guest={guest} host={host}");
        }
        if let Some(f) = resources.font_keyboard_stats() {
            eprintln!("ROM_DATA_USE resource={FONT_BANK_ID} dataset={} bus_reads={} glyph_bytes={} width_bytes={}", resources.dataset_sha256(), f.bus_reads, f.glyph_bytes, f.width_bytes);
            eprintln!("ROM_DATA_USE resource={NAME_KEYBOARD_ID} dataset={} index_bytes={} program_bytes={} row_width_bytes={} host_row_width_reads={}", resources.dataset_sha256(), f.keyboard_index_bytes, f.keyboard_program_bytes, f.keyboard_row_width_bytes, f.host_row_width_reads);
        }
        if let Some(t) = resources.text_catalog_stats() {
            eprintln!("ROM_DATA_USE resource={TEXT_CATALOGS_ID} dataset={} bus_reads={} source_bytes={} pointer_table_bytes={} string_bytes={} archive_index_bytes={} header_bytes={} encoded_control_bytes={} decoded_program_bytes={} glyph_bytes={} width_bytes={}",
                resources.dataset_sha256(),t.bus_reads,t.source_bytes,t.pointer_table_bytes,t.string_bytes,t.archive_index_bytes,t.header_bytes,t.encoded_control_bytes,t.decoded_program_bytes,t.glyph_bytes,t.width_bytes);
        }
        if let Some(texts) = resources.text_stats() {
            eprintln!("ROM_DATA_USE resource={SHORT_TEXTS_ID} dataset={} bus_reads={} source_bytes={} table_bytes={} string_bytes={} glyph_bytes={} width_bytes={}",
                resources.dataset_sha256(), texts.bus_reads, texts.source_bytes, texts.table_bytes, texts.string_bytes, texts.glyph_bytes, texts.width_bytes);
        }
        if let Some(transfers) = resources.transfer_stats() {
            eprintln!("ROM_DATA_USE resource={BG_TRANSFER_PROGRAMS_ID} dataset={} bus_reads={} source_bytes={} control_bytes={} tile_bytes={}",
                resources.dataset_sha256(), transfers.bus_reads, transfers.source_bytes, transfers.control_bytes, transfers.tile_bytes);
        }
    }
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("Cannot create {}: {e}", path.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("Cannot finish {}: {e}", path.display()))
}

/// Publish only into an absent directory. An existing valid dataset is reused;
/// invalid data produce an explicit error, never a silent ROM fallback.
pub fn extract(path: &Path, source: &mut Extraction) -> Result<&'static str, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            verify(path, source)?;
            return Ok("reused");
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Cannot inspect output directory: {e}")),
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| format!("Cannot create resources parent: {e}"))?;
    let temporary = tempfile::Builder::new()
        .prefix(".ffta-resources-")
        .tempdir_in(parent)
        .map_err(|e| format!("Cannot prepare resource directory: {e}"))?;
    write_dataset(temporary.path(), source, &mut write_synced)?;
    match fs::rename(temporary.path(), path) {
        Ok(()) => Ok("created"),
        // A concurrent writer may have published the same complete dataset.
        Err(_) if verify(path, source).is_ok() => Ok("reused"),
        Err(e) => Err(format!("Cannot publish resource directory: {e}")),
    }
}

fn write_dataset(
    path: &Path,
    source: &mut Extraction,
    write: &mut impl FnMut(&Path, &[u8]) -> Result<(), String>,
) -> Result<(), String> {
    let data = toml::to_string_pretty(&source.dimensions)
        .map_err(|e| format!("Cannot encode dimensions: {e}"))?;
    let background = source
        .background
        .as_ref()
        .ok_or("Missing background descriptor extraction")?;
    let bg_data = encode_background(background)?;
    let palette_data = encode_palettes(
        &source
            .palettes
            .as_ref()
            .ok_or("Missing base palette extraction")?
            .decoded,
    )?;
    let alternate_palette_data = encode_alternate_palettes(
        &source.alternate_palettes.as_ref().ok_or("Missing alternate palette extraction")?.decoded,
    )?;
    let packet_program_data = encode_packet_programs(
        &source.packet_programs.as_ref().ok_or("Missing packet program extraction")?.decoded,
    )?;
    let main_graphic_data = source.main_graphics.as_ref()
        .map(|catalog| encode_main_graphics(&catalog.decoded)).transpose()?;
    let transfer_data = source.transfers.as_ref()
        .map(|catalog| encode_transfers(&catalog.decoded)).transpose()?;
    let short_text_data = source.short_texts.as_ref()
        .map(|catalog| encode_short_texts(&catalog.decoded)).transpose()?;
    let text_catalog_data = source.text_catalogs.as_ref().map(|catalog| encode_text_catalogs(&catalog.decoded)).transpose()?;
    let font_bank_data = source.font_bank.as_ref().map(|catalog| encode_font_bank(&catalog.decoded)).transpose()?;
    let name_keyboard_data = source.name_keyboard.as_ref().map(|catalog| encode_name_keyboard(&catalog.decoded)).transpose()?;
    let obj_list_data = source.obj_lists.as_ref().map(|catalog| encode_obj_lists(&catalog.decoded)).transpose()?;
    let obj_animation_data = source.obj_animations.as_ref().map(|catalog| encode_obj_animations(&catalog.decoded)).transpose()?;
    let obj_graphic_data = source.obj_graphics.as_ref().map(|catalog| encode_obj_graphics(&catalog.decoded)).transpose()?;
    let ui_palette_data = source.ui_palettes.as_ref().map(|catalog| encode_ui_palettes(&catalog.decoded)).transpose()?;
    let ui_graphic_data = source.ui_graphics.as_ref().map(|catalog| encode_ui_graphics(&catalog.decoded)).transpose()?;
    let item_data = source.items.as_ref().map(|catalog|encode_items(&catalog.decoded)).transpose()?;
    let item_link_data=source.item_links.as_ref().map(|catalog|encode_item_links(&catalog.decoded)).transpose()?;
    let metadata16_data=source.metadata16.as_ref().map(|catalog|encode_metadata16(&catalog.decoded)).transpose()?;
    let metadata52_data=source.metadata52.as_ref().map(|catalog|encode_metadata52(&catalog.decoded)).transpose()?;
    let metadata4_data=source.metadata4.as_ref().map(|catalog|encode_metadata4(&catalog.decoded)).transpose()?;
    let metadata2_data=source.metadata2.as_ref().map(|catalog|encode_metadata2(&catalog.decoded)).transpose()?;
    let default48_data=source.default48.as_ref().map(|catalog|encode_default48(&catalog.decoded)).transpose()?;
    let constructor_alternates_data=source.constructor_alternates.as_ref().map(|catalog|encode_constructor_alternates(&catalog.decoded)).transpose()?;
    let metadata14_data=source.metadata14.as_ref().map(|catalog|encode_metadata14(&catalog.decoded)).transpose()?;
    let metadata12_data=source.metadata12.as_ref().map(|catalog|encode_metadata12(&catalog.decoded)).transpose()?;
    let global_pair_flags_data=source.global_pair_flags.as_ref().map(|catalog|encode_global_pair_flags(&catalog.decoded)).transpose()?;
    let evaluation_parameters_data=source.evaluation_parameters.as_ref().map(|catalog|encode_evaluation_parameters(&catalog.decoded)).transpose()?;
    let progression_rewards_data=source.progression_rewards.as_ref().map(|catalog|encode_progression_rewards(&catalog.decoded)).transpose()?;
    let progression_gates_data=source.progression_gates.as_ref().map(|catalog|encode_progression_gates(&catalog.decoded)).transpose()?;
    let progression_selection_gates_data=source.progression_selection_gates.as_ref().map(|catalog|encode_progression_selection_gates(&catalog.decoded)).transpose()?;
    let aggregate_flag_keys_data=source.aggregate_flag_keys.as_ref().map(|catalog|encode_aggregate_flag_keys(&catalog.decoded)).transpose()?;
    let trade_parameters_data=source.trade_parameters.as_ref().map(|catalog|encode_trade_parameters(&catalog.decoded)).transpose()?;
    let item_effect_parameters_data=source.item_effect_parameters.as_ref().map(|catalog|encode_item_effect_parameters(&catalog.decoded)).transpose()?;
    let audio_directory_data=source.audio_directory.as_ref().map(|catalog|encode_audio_directory(&catalog.decoded)).transpose()?;
    let audio_tones_data=source.audio_tones.as_ref().map(|catalog|encode_audio_tones(&catalog.decoded)).transpose()?;
    let audio_sequences_data=source.audio_sequences.as_ref().map(|catalog|encode_audio_sequences(&catalog.decoded)).transpose()?;
    let audio_driver_data=source.audio_driver.as_ref().map(|catalog|encode_audio_driver(&catalog.decoded)).transpose()?;
    let palette_effect_dispatch_data=source.palette_effect_dispatch.as_ref().map(|catalog|encode_palette_effect_dispatch(&catalog.decoded)).transpose()?;
    let palette_programs_data=source.palette_programs.as_ref().map(|catalog|encode_palette_programs(&catalog.decoded)).transpose()?;
    let blend_programs_data=source.blend_programs.as_ref().map(|catalog|encode_blend_programs(&catalog.decoded)).transpose()?;
    let task_descriptors_data=source.task_descriptors.as_ref().map(|catalog|encode_task_descriptors(&catalog.decoded)).transpose()?;
    let effect_event_directories_data=source.effect_event_directories.as_ref().map(|catalog|encode_effect_event_directories(&catalog.decoded)).transpose()?;
    let script_commands_data=source.script_commands.as_ref().map(|catalog|encode_script_commands(&catalog.decoded)).transpose()?;
    let script_archive_data=source.script_archive.as_ref().map(|catalog|encode_script_archive(&catalog.decoded)).transpose()?;
    let script_text_data=source.script_text.as_ref().map(|catalog|encode_script_text(&catalog.decoded)).transpose()?;
    let script_coordinates_data=source.script_coordinates.as_ref().map(|catalog|encode_script_coordinates(&catalog.decoded)).transpose()?;
    let ui_choice_lists_data=source.ui_choice_lists.as_ref().map(|catalog|encode_ui_choice_lists(&catalog.decoded)).transpose()?;
    let unit_obj_initial_pose_data=source.unit_obj_initial_pose.as_ref().map(|catalog|encode_unit_obj_initial_pose(&catalog.decoded)).transpose()?;
    let color_mode_palettes_data=source.color_mode_palettes.as_ref().map(|catalog|encode_color_mode_palettes(&catalog.decoded)).transpose()?;
    let unit_obj_equipment_data=source.unit_obj_equipment.as_ref().map(|catalog|encode_unit_obj_equipment(&catalog.decoded)).transpose()?;
    let unit_obj_normal_battle_data=source.unit_obj_normal_battle.as_ref().map(|catalog|encode_unit_obj_normal_battle(&catalog.decoded)).transpose()?;
    let unit_obj_alternate_battle_data=source.unit_obj_alternate_battle.as_ref().map(|catalog|encode_unit_obj_alternate_battle(&catalog.decoded)).transpose()?;
    let unit_obj_script_special_normal_data=source.unit_obj_script_special_normal.as_ref().map(|catalog|encode_unit_obj_script_special_normal(&catalog.decoded)).transpose()?;
    let unit_obj_script_special_alternate_data=source.unit_obj_script_special_alternate.as_ref().map(|catalog|encode_unit_obj_script_special_alternate(&catalog.decoded)).transpose()?;
    let unit_obj_script_override_data=source.unit_obj_script_override.as_ref().map(|catalog|encode_unit_obj_script_override(&catalog.decoded)).transpose()?;
    let unit_obj_script_registry_special_data=source.unit_obj_script_registry_special.as_ref().map(|catalog|encode_unit_obj_script_registry_special(&catalog.decoded)).transpose()?;
    let unit_obj_script_registry_generic_data=source.unit_obj_script_registry_generic.as_ref().map(|catalog|encode_unit_obj_script_registry_generic(&catalog.decoded)).transpose()?;
    let unit_obj_property_words_data=source.unit_obj_property_words.as_ref().map(|catalog|encode_unit_obj_property_words(&catalog.decoded)).transpose()?;
    let unit_obj_byte_property_words_data=source.unit_obj_byte_property_words.as_ref().map(|catalog|encode_unit_obj_byte_property_words(&catalog.decoded)).transpose()?;
    let unit_obj_ffff_reader_fields_data=source.unit_obj_ffff_reader_fields.as_ref().map(|catalog|encode_unit_obj_ffff_reader_fields(&catalog.decoded)).transpose()?;
    let slot_job_lookup_data=source.slot_job_lookup.as_ref().map(|catalog|encode_slot_job_lookup(&catalog.decoded)).transpose()?;
    let observed_task_parameters_data=source.observed_task_parameters.as_ref().map(|catalog|encode_observed_task_parameters(&catalog.decoded)).transpose()?;
    let constructor_obj_animation_data=source.constructor_obj_animation.as_ref().map(|catalog|encode_constructor_obj_animation(&catalog.decoded)).transpose()?;
    let task_cycle_deltas_data=source.task_cycle_deltas.as_ref().map(|catalog|encode_task_cycle_deltas(&catalog.decoded)).transpose()?;
    let ui_raw_transfers_data=source.ui_raw_transfers.as_ref().map(|c|encode_ui_raw_transfers(&c.decoded)).transpose()?;
    let dynamic_raw_fragments_data=source.dynamic_raw_fragments.as_ref().map(|c|encode_dynamic_raw_fragments(&c.decoded)).transpose()?;
    let conditional_four_byte_lookup_data=source.conditional_four_byte_lookup.as_ref().map(|c|encode_conditional_four_byte_lookup(&c.decoded)).transpose()?;
    let conditional_packed_lookup_data=source.conditional_packed_lookup.as_ref().map(|c|encode_conditional_packed_lookup(&c.decoded)).transpose()?;
    let conditional_descriptor_banks_data=source.conditional_descriptor_banks.as_ref().map(|c|encode_conditional_descriptor_banks(&c.decoded)).transpose()?;
    let conditional_descriptor_bank_zero_data=source.conditional_descriptor_bank_zero.as_ref().map(|c|encode_conditional_descriptor_bank_zero(&c.decoded)).transpose()?;
    let conditional_property_masks_data=source.conditional_property_masks.as_ref().map(|c|encode_conditional_property_masks(&c.decoded)).transpose()?;
    let compact_obj_animations_data=source.compact_obj_animations.as_ref().map(|c|encode_compact_obj_animations(&c.decoded)).transpose()?;
    let secondary_compact_obj_animations_data=source.secondary_compact_obj_animations.as_ref().map(|c|encode_secondary_compact_obj_animations(&c.decoded)).transpose()?;
    let tertiary_compact_obj_animations_data=source.tertiary_compact_obj_animations.as_ref().map(|c|encode_tertiary_compact_obj_animations(&c.decoded)).transpose()?;
    let quaternary_compact_obj_animations_data=source.quaternary_compact_obj_animations.as_ref().map(|c|encode_quaternary_compact_obj_animations(&c.decoded)).transpose()?;
    let quinary_compact_obj_animations_data=source.quinary_compact_obj_animations.as_ref().map(|c|encode_quinary_compact_obj_animations(&c.decoded)).transpose()?;
    let senary_compact_obj_animations_data=source.senary_compact_obj_animations.as_ref().map(|c|encode_senary_compact_obj_animations(&c.decoded)).transpose()?;
    let indexed_graphics_data=source.indexed_graphics.as_ref().map(|c|encode_indexed_graphics(&c.decoded)).transpose()?;
    let world_records38_data=source.world_records38.as_ref().map(|c|encode_world_records38(&c.decoded)).transpose()?;
    let parameter_records27_data=source.parameter_records27.as_ref().map(|c|encode_parameter_records27(&c.decoded)).transpose()?;
    let event_records12_data=source.event_records12.as_ref().map(|c|encode_event_records12(&c.decoded)).transpose()?;
    let selection_byte_tables_data=source.selection_byte_tables.as_ref().map(|c|encode_selection_byte_tables(&c.decoded)).transpose()?;
    let mission_records70_data=source.mission_records70.as_ref().map(|c|encode_mission_records70(&c.decoded)).transpose()?;
    let conditional_records8_data=source.conditional_records8.as_ref().map(|c|encode_conditional_records8(&c.decoded)).transpose()?;
    let records4_data=source.records4.as_ref().map(|c|encode_records4(&c.decoded)).transpose()?;
    let metadata28_data=source.metadata28.as_ref().map(|c|encode_metadata28(&c.decoded)).transpose()?;
    let encounter_coordinates_data=source.encounter_coordinates.as_ref().map(|c|encode_encounter_coordinates(&c.decoded)).transpose()?;
    let encounter_unit_templates_data=source.encounter_unit_templates.as_ref().map(|catalog|encode_encounter_unit_templates(&catalog.decoded)).transpose()?;
    let encounter_headers_data=source.encounter_headers.as_ref().map(|catalog|encode_encounter_headers(&catalog.decoded)).transpose()?;
    let world_roads_data=source.world_roads.as_ref().map(|catalog|encode_world_roads(&catalog.decoded)).transpose()?;
    let world_nodes_data=source.world_nodes.as_ref().map(|catalog|encode_world_nodes(&catalog.decoded)).transpose()?;
    let word_task_obj_animations_data=source.word_task_obj_animations.as_ref().map(|catalog|encode_word_task_obj_animations(&catalog.decoded)).transpose()?;
    let map_raster_palettes_data=source.map_raster_palettes.as_ref().map(|catalog|encode_map_raster_palettes(&catalog.decoded)).transpose()?;
    let auxiliary_commands_data=source.auxiliary_scripts.as_ref().map(|catalog|encode_auxiliary_commands(&catalog.commands)).transpose()?;
    let auxiliary_archive_data=source.auxiliary_scripts.as_ref().map(|catalog|encode_auxiliary_archive(&catalog.decoded)).transpose()?;
    let mut files: Vec<&[u8]> = vec![data.as_bytes(), &bg_data, &palette_data, &alternate_palette_data, &packet_program_data];
    if let Some(bytes) = &main_graphic_data { files.push(bytes); }
    if let Some(bytes) = &transfer_data { files.push(bytes); }
    if let Some(bytes) = &short_text_data { files.push(bytes); }
    if let Some(bytes) = &text_catalog_data { files.push(bytes); }
    if let Some(bytes) = &font_bank_data { files.push(bytes); }
    if let Some(bytes) = &name_keyboard_data { files.push(bytes); }
    if let Some(bytes) = &obj_list_data { files.push(bytes); }
    if let Some(bytes) = &obj_animation_data { files.push(bytes); }
    if let Some(bytes) = &obj_graphic_data { files.push(bytes); }
    if let Some(bytes) = &ui_palette_data { files.push(bytes); }
    if let Some(bytes) = &ui_graphic_data { files.push(bytes); }
    if let Some(bytes) = &item_data {files.push(bytes);}
    if let Some(bytes) = &item_link_data {files.push(bytes);}
    if let Some(bytes) = &metadata16_data {files.push(bytes);}
    if let Some(bytes) = &metadata52_data {files.push(bytes);}
    if let Some(bytes) = &metadata4_data {files.push(bytes);}
    if let Some(bytes) = &metadata2_data {files.push(bytes);}
    if let Some(bytes) = &default48_data {files.push(bytes);}
    if let Some(bytes) = &constructor_alternates_data {files.push(bytes);}
    if let Some(bytes) = &metadata14_data {files.push(bytes);}
    if let Some(bytes) = &metadata12_data {files.push(bytes);}
    if let Some(bytes) = &global_pair_flags_data {files.push(bytes);}
    if let Some(bytes) = &evaluation_parameters_data {files.push(bytes);}
    if let Some(bytes) = &progression_rewards_data {files.push(bytes);}
    if let Some(bytes) = &progression_gates_data {files.push(bytes);}
    if let Some(bytes) = &progression_selection_gates_data {files.push(bytes);}
    if let Some(bytes) = &aggregate_flag_keys_data {files.push(bytes);}
    if let Some(bytes) = &trade_parameters_data {files.push(bytes);}
    if let Some(bytes) = &item_effect_parameters_data {files.push(bytes);}
    if let Some(bytes) = &audio_directory_data {files.push(bytes);}
    if let Some(bytes) = &audio_tones_data {files.push(bytes);}
    if let Some(bytes) = &audio_sequences_data {files.push(bytes);}
    if let Some(bytes) = &audio_driver_data {files.push(bytes);}
    if let Some(bytes) = &palette_effect_dispatch_data {files.push(bytes);}
    if let Some(bytes) = &palette_programs_data {files.push(bytes);}
    if let Some(bytes) = &blend_programs_data {files.push(bytes);}
    if let Some(bytes) = &task_descriptors_data {files.push(bytes);}
    if let Some(bytes) = &effect_event_directories_data {files.push(bytes);}
    if let Some(bytes) = &script_commands_data {files.push(bytes);}
    if let Some(bytes) = &script_archive_data {files.push(bytes);}
    if let Some(bytes) = &script_text_data {files.push(bytes);}
    if let Some(bytes) = &script_coordinates_data {files.push(bytes);}
    if let Some(bytes) = &auxiliary_commands_data {files.push(bytes);}
    if let Some(bytes) = &auxiliary_archive_data {files.push(bytes);}
    if let Some(bytes) = &map_raster_palettes_data {files.push(bytes);}
    if let Some(bytes) = &ui_choice_lists_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_initial_pose_data {files.push(bytes);}
    if let Some(bytes) = &color_mode_palettes_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_equipment_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_normal_battle_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_alternate_battle_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_script_special_normal_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_script_special_alternate_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_script_override_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_script_registry_special_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_script_registry_generic_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_property_words_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_byte_property_words_data {files.push(bytes);}
    if let Some(bytes) = &unit_obj_ffff_reader_fields_data {files.push(bytes);}
    if let Some(bytes) = &slot_job_lookup_data {files.push(bytes);}
    if let Some(bytes) = &observed_task_parameters_data {files.push(bytes);}
    if let Some(bytes) = &constructor_obj_animation_data {files.push(bytes);}
    if let Some(bytes) = &task_cycle_deltas_data {files.push(bytes);}
    if let Some(bytes) = &word_task_obj_animations_data {files.push(bytes);}
    if let Some(bytes) = &world_nodes_data {files.push(bytes);}
    if let Some(bytes) = &world_roads_data {files.push(bytes);}
    if let Some(bytes) = &encounter_headers_data {files.push(bytes);}
    if let Some(bytes) = &encounter_unit_templates_data {files.push(bytes);}
    if let Some(bytes)=&encounter_coordinates_data {files.push(bytes);}
    if let Some(bytes)=&metadata28_data {files.push(bytes);}
    if let Some(bytes)=&records4_data {files.push(bytes);}
    if let Some(bytes)=&conditional_records8_data {files.push(bytes);}
    if let Some(bytes)=&mission_records70_data {files.push(bytes);}
    if let Some(bytes)=&selection_byte_tables_data {files.push(bytes);}
    if let Some(bytes)=&event_records12_data {files.push(bytes);}
    if let Some(bytes)=&parameter_records27_data {files.push(bytes);}
    if let Some(bytes)=&world_records38_data {files.push(bytes);}
    if let Some(bytes)=&indexed_graphics_data {files.push(bytes);}
    if let Some(bytes)=&ui_raw_transfers_data {files.push(bytes);}
    if let Some(bytes)=&dynamic_raw_fragments_data {files.push(bytes);}
    if let Some(bytes)=&conditional_four_byte_lookup_data {files.push(bytes);}
    if let Some(bytes)=&conditional_packed_lookup_data {files.push(bytes);}
    if let Some(bytes)=&conditional_descriptor_banks_data {files.push(bytes);}
    if let Some(bytes)=&conditional_descriptor_bank_zero_data {files.push(bytes);}
    if let Some(bytes)=&conditional_property_masks_data {files.push(bytes);}
    if let Some(bytes)=&compact_obj_animations_data {files.push(bytes);}
    if let Some(bytes)=&secondary_compact_obj_animations_data {files.push(bytes);}
    if let Some(bytes)=&tertiary_compact_obj_animations_data {files.push(bytes);}
    if let Some(bytes)=&quaternary_compact_obj_animations_data {files.push(bytes);}
    if let Some(bytes)=&quinary_compact_obj_animations_data {files.push(bytes);}
    if let Some(bytes)=&senary_compact_obj_animations_data {files.push(bytes);}
    source.set_file_hashes(&files)?;
    let manifest = toml::to_string_pretty(&source.manifest)
        .map_err(|e| format!("Cannot encode manifest: {e}"))?;
    check_dataset_budget(
        manifest.len(),
        data.len(),
        bg_data.len(),
        palette_data.len(),
        alternate_palette_data.len(),
        packet_program_data.len(),
        main_graphic_data.as_ref().map_or(0, Vec::len),
        transfer_data.as_ref().map_or(0, Vec::len),
        short_text_data.as_ref().map_or(0, Vec::len),
        text_catalog_data.as_ref().map_or(0, Vec::len),
        font_bank_data.as_ref().map_or(0, Vec::len),
        name_keyboard_data.as_ref().map_or(0, Vec::len),
        obj_list_data.as_ref().map_or(0, Vec::len),
        obj_animation_data.as_ref().map_or(0, Vec::len),
        obj_graphic_data.as_ref().map_or(0, Vec::len),
        ui_palette_data.as_ref().map_or(0, Vec::len),
        ui_graphic_data.as_ref().map_or(0, Vec::len),
        item_data.as_ref().map_or(0, Vec::len),
        item_link_data.as_ref().map_or(0, Vec::len),
        metadata16_data.as_ref().map_or(0, Vec::len),
        metadata52_data.as_ref().map_or(0, Vec::len),
        metadata4_data.as_ref().map_or(0, Vec::len),
        metadata2_data.as_ref().map_or(0, Vec::len),
        default48_data.as_ref().map_or(0, Vec::len),
        constructor_alternates_data.as_ref().map_or(0, Vec::len),
        metadata14_data.as_ref().map_or(0, Vec::len),
        metadata12_data.as_ref().map_or(0, Vec::len),
        global_pair_flags_data.as_ref().map_or(0, Vec::len),
        evaluation_parameters_data.as_ref().map_or(0, Vec::len),
        progression_rewards_data.as_ref().map_or(0, Vec::len),
        progression_gates_data.as_ref().map_or(0, Vec::len),
        progression_selection_gates_data.as_ref().map_or(0, Vec::len),
        aggregate_flag_keys_data.as_ref().map_or(0, Vec::len),
        trade_parameters_data.as_ref().map_or(0, Vec::len),
        item_effect_parameters_data.as_ref().map_or(0, Vec::len),
        audio_directory_data.as_ref().map_or(0, Vec::len),
        audio_tones_data.as_ref().map_or(0, Vec::len),
        audio_sequences_data.as_ref().map_or(0, Vec::len),
        audio_driver_data.as_ref().map_or(0, Vec::len),
        palette_effect_dispatch_data.as_ref().map_or(0, Vec::len),
        palette_programs_data.as_ref().map_or(0, Vec::len),
        blend_programs_data.as_ref().map_or(0, Vec::len),
        task_descriptors_data.as_ref().map_or(0, Vec::len),
        effect_event_directories_data.as_ref().map_or(0, Vec::len),
        script_commands_data.as_ref().map_or(0, Vec::len),
        script_archive_data.as_ref().map_or(0, Vec::len),
        script_text_data.as_ref().map_or(0, Vec::len),
        script_coordinates_data.as_ref().map_or(0, Vec::len),
        ui_choice_lists_data.as_ref().map_or(0, Vec::len),
        unit_obj_initial_pose_data.as_ref().map_or(0, Vec::len),
        color_mode_palettes_data.as_ref().map_or(0, Vec::len),
        unit_obj_equipment_data.as_ref().map_or(0, Vec::len),
        unit_obj_normal_battle_data.as_ref().map_or(0, Vec::len),
        unit_obj_alternate_battle_data.as_ref().map_or(0, Vec::len),
        unit_obj_script_special_normal_data.as_ref().map_or(0, Vec::len),
        unit_obj_script_special_alternate_data.as_ref().map_or(0, Vec::len),
        unit_obj_script_override_data.as_ref().map_or(0, Vec::len),
        unit_obj_script_registry_special_data.as_ref().map_or(0, Vec::len),
        unit_obj_script_registry_generic_data.as_ref().map_or(0, Vec::len),
        unit_obj_property_words_data.as_ref().map_or(0, Vec::len),
        unit_obj_byte_property_words_data.as_ref().map_or(0, Vec::len),
        unit_obj_ffff_reader_fields_data.as_ref().map_or(0, Vec::len),
        slot_job_lookup_data.as_ref().map_or(0, Vec::len),
        observed_task_parameters_data.as_ref().map_or(0, Vec::len),
        constructor_obj_animation_data.as_ref().map_or(0, Vec::len),
        task_cycle_deltas_data.as_ref().map_or(0, Vec::len),
        word_task_obj_animations_data.as_ref().map_or(0, Vec::len),
        world_nodes_data.as_ref().map_or(0, Vec::len),
        world_roads_data.as_ref().map_or(0, Vec::len),
        encounter_headers_data.as_ref().map_or(0, Vec::len),
        encounter_unit_templates_data.as_ref().map_or(0, Vec::len),
        encounter_coordinates_data.as_ref().map_or(0, Vec::len),
        metadata28_data.as_ref().map_or(0, Vec::len),
        records4_data.as_ref().map_or(0, Vec::len),
        conditional_records8_data.as_ref().map_or(0, Vec::len),
        mission_records70_data.as_ref().map_or(0, Vec::len),
        selection_byte_tables_data.as_ref().map_or(0, Vec::len),
        event_records12_data.as_ref().map_or(0, Vec::len),
        parameter_records27_data.as_ref().map_or(0, Vec::len),
        world_records38_data.as_ref().map_or(0, Vec::len),
        indexed_graphics_data.as_ref().map_or(0, Vec::len),
        ui_raw_transfers_data.as_ref().map_or(0, Vec::len),
        dynamic_raw_fragments_data.as_ref().map_or(0, Vec::len),
        conditional_four_byte_lookup_data.as_ref().map_or(0, Vec::len),
        conditional_packed_lookup_data.as_ref().map_or(0, Vec::len),
        conditional_descriptor_banks_data.as_ref().map_or(0, Vec::len),
        conditional_descriptor_bank_zero_data.as_ref().map_or(0, Vec::len),
        conditional_property_masks_data.as_ref().map_or(0, Vec::len),
        compact_obj_animations_data.as_ref().map_or(0, Vec::len),
        secondary_compact_obj_animations_data.as_ref().map_or(0, Vec::len),
        tertiary_compact_obj_animations_data.as_ref().map_or(0, Vec::len),
        quaternary_compact_obj_animations_data.as_ref().map_or(0, Vec::len),
        quinary_compact_obj_animations_data.as_ref().map_or(0, Vec::len),
        senary_compact_obj_animations_data.as_ref().map_or(0, Vec::len),
        map_raster_palettes_data.as_ref().map_or(0, Vec::len),
        auxiliary_commands_data.as_ref().map_or(0, Vec::len),
        auxiliary_archive_data.as_ref().map_or(0, Vec::len),
    )?;
    write(&path.join(OBJ_DIMENSIONS_FILE), data.as_bytes())?;
    write(&path.join(BG_DESCRIPTORS_FILE), &bg_data)?;
    write(&path.join(BG_BASE_PALETTES_FILE), &palette_data)?;
    write(&path.join(BG_ALTERNATE_PALETTES_FILE), &alternate_palette_data)?;
    write(&path.join(BG_PACKET_PROGRAMS_FILE), &packet_program_data)?;
    if let Some(bytes) = &main_graphic_data { write(&path.join(BG_MAIN_GRAPHICS_FILE), bytes)?; }
    if let Some(bytes) = &transfer_data { write(&path.join(BG_TRANSFER_PROGRAMS_FILE), bytes)?; }
    if let Some(bytes) = &short_text_data { write(&path.join(SHORT_TEXTS_FILE), bytes)?; }
    if let Some(bytes) = &text_catalog_data { write(&path.join(TEXT_CATALOGS_FILE), bytes)?; }
    if let Some(bytes) = &font_bank_data { write(&path.join(FONT_BANK_FILE), bytes)?; }
    if let Some(bytes) = &name_keyboard_data { write(&path.join(NAME_KEYBOARD_FILE), bytes)?; }
    if let Some(bytes) = &obj_list_data { write(&path.join(OBJ_STATIC_LISTS_FILE), bytes)?; }
    if let Some(bytes) = &obj_animation_data { write(&path.join(OBJ_ANIMATION_BANKS_FILE), bytes)?; }
    if let Some(bytes) = &obj_graphic_data { write(&path.join(OBJ_CUSTOM_GRAPHICS_FILE), bytes)?; }
    if let Some(bytes) = &ui_palette_data { write(&path.join(UI_RAW_PALETTES_FILE), bytes)?; }
    if let Some(bytes) = &ui_graphic_data { write(&path.join(UI_CUSTOM_GRAPHICS_FILE), bytes)?; }
    if let Some(bytes) = &item_data {write(&path.join(ITEM_RECORDS_FILE),bytes)?;}
    if let Some(bytes) = &item_link_data {write(&path.join(ITEM_LINKS_FILE),bytes)?;}
    if let Some(bytes) = &metadata16_data {write(&path.join(METADATA16_FILE),bytes)?;}
    if let Some(bytes) = &metadata52_data {write(&path.join(METADATA52_FILE),bytes)?;}
    if let Some(bytes) = &metadata4_data {write(&path.join(METADATA4_FILE),bytes)?;}
    if let Some(bytes) = &metadata2_data {write(&path.join(METADATA2_FILE),bytes)?;}
    if let Some(bytes) = &default48_data {write(&path.join(DEFAULT48_FILE),bytes)?;}
    if let Some(bytes) = &constructor_alternates_data {write(&path.join(CONSTRUCTOR_ALTERNATES_FILE),bytes)?;}
    if let Some(bytes) = &metadata14_data {write(&path.join(METADATA14_FILE),bytes)?;}
    if let Some(bytes) = &metadata12_data {write(&path.join(METADATA12_FILE),bytes)?;}
    if let Some(bytes) = &global_pair_flags_data {write(&path.join(GLOBAL_PAIR_FLAGS_FILE),bytes)?;}
    if let Some(bytes) = &evaluation_parameters_data {write(&path.join(EVALUATION_PARAMETERS_FILE),bytes)?;}
    if let Some(bytes) = &progression_rewards_data {write(&path.join(PROGRESSION_REWARDS_FILE),bytes)?;}
    if let Some(bytes) = &progression_gates_data {write(&path.join(PROGRESSION_GATES_FILE),bytes)?;}
    if let Some(bytes) = &progression_selection_gates_data {write(&path.join(PROGRESSION_SELECTION_GATES_FILE),bytes)?;}
    if let Some(bytes) = &aggregate_flag_keys_data {write(&path.join(AGGREGATE_FLAG_KEYS_FILE),bytes)?;}
    if let Some(bytes) = &trade_parameters_data {write(&path.join(TRADE_PARAMETERS_FILE),bytes)?;}
    if let Some(bytes) = &item_effect_parameters_data {write(&path.join(ITEM_EFFECT_PARAMETERS_FILE),bytes)?;}
    if let Some(bytes) = &audio_directory_data {write(&path.join(AUDIO_DIRECTORY_FILE),bytes)?;}
    if let Some(bytes) = &audio_tones_data {write(&path.join(AUDIO_TONES_FILE),bytes)?;}
    if let Some(bytes) = &audio_sequences_data {write(&path.join(AUDIO_SEQUENCES_FILE),bytes)?;}
    if let Some(bytes) = &audio_driver_data {write(&path.join(AUDIO_DRIVER_FILE),bytes)?;}
    if let Some(bytes) = &palette_effect_dispatch_data {write(&path.join(PALETTE_EFFECT_DISPATCH_FILE),bytes)?;}
    if let Some(bytes) = &palette_programs_data {write(&path.join(PALETTE_PROGRAMS_FILE),bytes)?;}
    if let Some(bytes) = &blend_programs_data {write(&path.join(BLEND_PROGRAMS_FILE),bytes)?;}
    if let Some(bytes) = &task_descriptors_data {write(&path.join(TASK_DESCRIPTORS_FILE),bytes)?;}
    if let Some(bytes) = &effect_event_directories_data {write(&path.join(EFFECT_EVENT_DIRECTORIES_FILE),bytes)?;}
    if let Some(bytes) = &script_commands_data {write(&path.join(SCRIPT_COMMANDS_FILE),bytes)?;}
    if let Some(bytes) = &script_archive_data {write(&path.join(SCRIPT_ARCHIVE_FILE),bytes)?;}
    if let Some(bytes) = &script_text_data {write(&path.join(SCRIPT_TEXT_FILE),bytes)?;}
    if let Some(bytes) = &script_coordinates_data {write(&path.join(SCRIPT_COORDINATES_FILE),bytes)?;}
    if let Some(bytes) = &ui_choice_lists_data {write(&path.join(UI_CHOICE_LISTS_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_initial_pose_data {write(&path.join(UNIT_OBJ_INITIAL_POSE_FILE),bytes)?;}
    if let Some(bytes) = &color_mode_palettes_data {write(&path.join(COLOR_MODE_PALETTES_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_equipment_data {write(&path.join(UNIT_OBJ_EQUIPMENT_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_normal_battle_data {write(&path.join(UNIT_OBJ_NORMAL_BATTLE_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_alternate_battle_data {write(&path.join(UNIT_OBJ_ALTERNATE_BATTLE_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_script_special_normal_data {write(&path.join(UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_script_special_alternate_data {write(&path.join(UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_script_override_data {write(&path.join(UNIT_OBJ_SCRIPT_OVERRIDE_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_script_registry_special_data {write(&path.join(UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_script_registry_generic_data {write(&path.join(UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_property_words_data {write(&path.join(UNIT_OBJ_PROPERTY_WORDS_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_byte_property_words_data {write(&path.join(UNIT_OBJ_BYTE_PROPERTY_WORDS_FILE),bytes)?;}
    if let Some(bytes) = &unit_obj_ffff_reader_fields_data {write(&path.join(UNIT_OBJ_FFFF_READER_FIELDS_FILE),bytes)?;}
    if let Some(bytes) = &slot_job_lookup_data {write(&path.join(SLOT_JOB_LOOKUP_FILE),bytes)?;}
    if let Some(bytes) = &observed_task_parameters_data {write(&path.join(OBSERVED_TASK_PARAMETERS_FILE),bytes)?;}
    if let Some(bytes) = &constructor_obj_animation_data {write(&path.join(CONSTRUCTOR_OBJ_ANIMATION_FILE),bytes)?;}
    if let Some(bytes) = &task_cycle_deltas_data {write(&path.join(TASK_CYCLE_DELTAS_FILE),bytes)?;}
    if let Some(bytes) = &word_task_obj_animations_data {write(&path.join(WORD_TASK_OBJ_ANIMATIONS_FILE),bytes)?;}
    if let Some(bytes) = &world_nodes_data {write(&path.join(WORLD_NODES_FILE),bytes)?;}
    if let Some(bytes) = &world_roads_data {write(&path.join(WORLD_ROADS_FILE),bytes)?;}
    if let Some(bytes) = &encounter_headers_data {write(&path.join(ENCOUNTER_HEADERS_FILE),bytes)?;}
    if let Some(bytes) = &encounter_unit_templates_data {write(&path.join(ENCOUNTER_UNIT_TEMPLATES_FILE),bytes)?;}
    if let Some(bytes)=&encounter_coordinates_data {write(&path.join(ENCOUNTER_COORDINATES_FILE),bytes)?;}
    if let Some(bytes)=&metadata28_data {write(&path.join(METADATA28_FILE),bytes)?;}
    if let Some(bytes)=&records4_data {write(&path.join(RECORDS4_FILE),bytes)?;}
    if let Some(bytes)=&conditional_records8_data {write(&path.join(CONDITIONAL_RECORDS8_FILE),bytes)?;}
    if let Some(bytes)=&mission_records70_data {write(&path.join(MISSION_RECORDS70_FILE),bytes)?;}
    if let Some(bytes)=&selection_byte_tables_data {write(&path.join(SELECTION_BYTE_TABLES_FILE),bytes)?;}
    if let Some(bytes)=&event_records12_data {write(&path.join(EVENT_RECORDS12_FILE),bytes)?;}
    if let Some(bytes)=&parameter_records27_data {write(&path.join(PARAMETER_RECORDS27_FILE),bytes)?;}
    if let Some(bytes)=&world_records38_data {write(&path.join(WORLD_RECORDS38_FILE),bytes)?;}
    if let Some(bytes)=&indexed_graphics_data {write(&path.join(INDEXED_GRAPHICS_FILE),bytes)?;}
    if let Some(bytes)=&ui_raw_transfers_data {write(&path.join(UI_RAW_TRANSFERS_FILE),bytes)?;}
    if let Some(bytes)=&dynamic_raw_fragments_data {write(&path.join(DYNAMIC_RAW_FRAGMENTS_FILE),bytes)?;}
    if let Some(bytes)=&conditional_four_byte_lookup_data {write(&path.join(CONDITIONAL_FOUR_BYTE_LOOKUP_FILE),bytes)?;}
    if let Some(bytes)=&conditional_packed_lookup_data {write(&path.join(CONDITIONAL_PACKED_LOOKUP_FILE),bytes)?;}
    if let Some(bytes)=&conditional_descriptor_banks_data {write(&path.join(CONDITIONAL_DESCRIPTOR_BANKS_FILE),bytes)?;}
    if let Some(bytes)=&conditional_descriptor_bank_zero_data {write(&path.join(CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE),bytes)?;}
    if let Some(bytes)=&conditional_property_masks_data {write(&path.join(CONDITIONAL_PROPERTY_MASKS_FILE),bytes)?;}
    if let Some(bytes)=&compact_obj_animations_data {write(&path.join(COMPACT_OBJ_ANIMATIONS_FILE),bytes)?;}
    if let Some(bytes)=&secondary_compact_obj_animations_data {write(&path.join(SECONDARY_COMPACT_OBJ_ANIMATIONS_FILE),bytes)?;}
    if let Some(bytes)=&tertiary_compact_obj_animations_data {write(&path.join(TERTIARY_COMPACT_OBJ_ANIMATIONS_FILE),bytes)?;}
    if let Some(bytes)=&quaternary_compact_obj_animations_data {write(&path.join(QUATERNARY_COMPACT_OBJ_ANIMATIONS_FILE),bytes)?;}
    if let Some(bytes)=&quinary_compact_obj_animations_data {write(&path.join(QUINARY_COMPACT_OBJ_ANIMATIONS_FILE),bytes)?;}
    if let Some(bytes)=&senary_compact_obj_animations_data {write(&path.join(SENARY_COMPACT_OBJ_ANIMATIONS_FILE),bytes)?;}
    if let Some(bytes) = &map_raster_palettes_data {write(&path.join(MAP_RASTER_PALETTES_FILE),bytes)?;}
    if let Some(bytes) = &auxiliary_commands_data {write(&path.join(AUXILIARY_COMMANDS_FILE),bytes)?;}
    if let Some(bytes) = &auxiliary_archive_data {write(&path.join(AUXILIARY_ARCHIVE_FILE),bytes)?;}
    write(&path.join(MANIFEST_FILE), manifest.as_bytes())?;
    verify(path, source)?;
    Ok(())
}

pub fn cmd(args: &[String]) -> Result<(), String> {
    let operation = args.first().map(String::as_str);
    if !matches!(
        (operation, args.len()),
        (Some("extract" | "verify"), 3) | (Some("prepare"), 2 | 3)
    ) {
        return Err("usage: recomp rom-data <extract|verify> <rom> <dataset-directory> OR rom-data prepare <rom> [resources-root]".into());
    }
    let rom = fs::read(&args[1]).map_err(|e| format!("Cannot read ROM: {e}"))?;
    let mut source = Extraction::from_rom(&rom)?;
    if operation == Some("prepare") {
        let root = args
            .get(2)
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(preparation::storage_root)?;
        let prepared = preparation::prepare(
            &root,
            &mut source,
            &mut preparation::console_progress(false),
        )?;
        println!("ROM_DATA action={} profile={} source={} schema={} decoded={} runtime=offline directory={}",
            prepared.action, source.manifest.profile, source.manifest.source_rom_sha256,
            source.manifest.schema, source.manifest.coverage.join(","), prepared.directory.display());
        return Ok(());
    }
    let path = Path::new(&args[2]);
    let action = if args[0] == "extract" {
        extract(path, &mut source)?
    } else {
        verify(path, &source)?;
        "verified"
    };
    println!("ROM_DATA action={action} profile={} source={} schema={} decoded={} files={} runtime=offline",
        source.manifest.profile, source.manifest.source_rom_sha256, source.manifest.schema,
        source.manifest.coverage.join(","), source.manifest.resources.len() + 1);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic structure, deliberately not a copy of a game's dimension table.
    pub(super) fn fixture() -> Extraction {
        use gba_core::rom_data::*;
        let dimensions = ObjDimensions {
            entries: std::array::from_fn(|i| ObjSize {
                width: 8 << (i % 4),
                height: 8 << ((i + 1) % 4),
            }),
        };
        let absent = bg_descriptors::TransferLink {
            control_source: 0,
            data_source: 0,
            destination: 0,
        };
        let mut background = Descriptors {
            entries: vec![
                bg_descriptors::Descriptor {
                    graphics_source: 0x4000,
                    paired_planes_source: 0x4100,
                    halfword_planes_source: 0x4200,
                    palette_source: 0x4300,
                    auxiliary_patches_source: 0x4400,
                    transfer: absent.clone(),
                    extra_transfers: std::array::from_fn(|_| absent.clone()),
                    variant_source: 0,
                    selectors: [1, 2],
                    alternate_palette_key: 3,
                    trailing_byte: 0,
                };
                bg_descriptors::BASE_RECORDS
            ],
        };
        for (id, entry) in background.entries.iter_mut().enumerate() {
            entry.alternate_palette_key = if id < bg_alternate_palettes::BASE_KEYS {id as u8} else {255};
        }
        let bg_hash = gba_core::ffta_rom::digest(&background.source_bytes().unwrap());
        let decoded_palettes = Palettes {
            entries: vec![
                bg_palettes::Palette {
                    banks: vec![[0x8123; 16]; 4],
                    encoded_source: [
                        vec![0, 128, 0, 0],
                        (0..64).flat_map(|_| 0x8123u16.to_le_bytes()).collect()
                    ]
                    .concat(),
                };
                bg_descriptors::BASE_RECORDS
            ],
        };
        let palette_sources: Vec<_> = (0..bg_descriptors::BASE_RECORDS)
            .map(|id| bg_palettes::PaletteSource {
                range: SourceRange {
                    offset: 0x4400,
                    length: 132,
                    sha256: "synthetic-palette-source".into(),
                    role: format!("base-palette/{id}"),
                },
                encoding: bg_palettes::Encoding::Raw,
            })
            .collect();
        let palette_ranges = palette_sources
            .iter()
            .map(|source| source.range.clone())
            .collect();
        let alternate_tables = std::array::from_fn(|group| SourceRange {
            offset: 0x6000 + group as u32 * 0x2000, length: 314,
            sha256: format!("synthetic-alternate-links-{group}"),
            role: format!("alternate-palette-links/{group}"),
        });
        let alternate_sources: Vec<_> = (0..314).map(|index| SourceRange {
            offset: alternate_tables[index / 157].offset + 316 + (index % 157) as u32 * 8,
            length: 6, sha256: "synthetic-alternate-source".into(),
            role: format!("alternate-palette/{}/{}", index / 157, index % 157),
        }).collect();
        let alternate_ranges = alternate_tables.iter().chain(&alternate_sources).cloned().collect();
        let alternate_palettes = bg_alternate_palettes::Catalog {
            tables: alternate_tables,
            sources: alternate_sources,
            decoded: AlternatePalettes {
                descriptor_keys: background.entries.iter().map(|e| e.alternate_palette_key).collect(),
                relative_sources: std::array::from_fn(|_| (0..157).map(|i| (316+i*8) as u16).collect()),
                catalogs: std::array::from_fn(|group| vec![bg_alternate_palettes::Palette::decode(&[0,0,1,0,if group==0 {1}else{2},253]).unwrap().0;157]),
            },
        };
        let packet_family = |role, relative, body: &[u8]| {
            let source = [vec![1, 255, 255, 255], body.to_vec()].concat();
            bg_packet_data::Family {
                descriptor_programs: vec![0; bg_descriptors::BASE_RECORDS],
                programs: vec![bg_packet_data::Program::decode(&source, role, relative).unwrap().0],
            }
        };
        let mut paired_body = vec![16, 0, 0, 128, 0, 0, 1, 0, 0, 0];
        paired_body.resize(16, 0);
        paired_body.extend_from_slice(&[7, 0]);
        let halfword_body = [0, 0, 2, 0, 0x23, 0x81, 0, 0];
        let packet_decoded = bg_packet_data::Programs {
            paired: packet_family(bg_packet_data::Role::Paired, 0x4100, &paired_body),
            halfwords: packet_family(bg_packet_data::Role::Halfwords, 0x4200, &halfword_body),
            auxiliary: packet_family(bg_packet_data::Role::Auxiliary, 0x4400, &halfword_body),
        };
        let packet_sources: Vec<_> = bg_packet_data::Role::ALL.into_iter().map(|role| {
            let program = &packet_decoded.family(role).programs[0];
            SourceRange {
                offset: 0x100 + program.relative_source,
                length: program.encoded_source.len() as u32,
                sha256: gba_core::ffta_rom::digest(&program.encoded_source),
                role: format!("bg-packet-program/{}/0", role.name()),
            }
        }).collect();
        let packet_ranges = packet_sources.clone();
        let packet_programs = bg_packet_data::Catalog { decoded: packet_decoded, sources: packet_sources };
        Extraction {
            manifest: Manifest {
                schema: SCHEMA,
                extractor_revision: EXTRACTOR_REV,
                decoder_revision: DECODER_REV,
                source_rom_sha256: "synthetic-source".into(),
                source_rom_size: 0x100000,
                profile: "synthetic".into(),
                coverage: vec![
                    OBJ_DIMENSIONS_ID.into(),
                    BG_DESCRIPTORS_ID.into(),
                    BG_BASE_PALETTES_ID.into(),
                    BG_ALTERNATE_PALETTES_ID.into(),
                    BG_PACKET_PROGRAMS_ID.into(),
                ],
                resources: vec![
                    Resource {
                        id: OBJ_DIMENSIONS_ID.into(),
                        path: OBJ_DIMENSIONS_FILE.into(),
                        format: "obj-dimensions-v1".into(),
                        sha256: String::new(),
                        dependencies: vec![],
                        sources: vec![SourceRange {
                            offset: 0,
                            length: 48,
                            sha256: gba_core::ffta_rom::digest(&dimensions.source_bytes()),
                            role: "synthetic".into(),
                        }],
                    },
                    Resource {
                        id: BG_DESCRIPTORS_ID.into(),
                        path: BG_DESCRIPTORS_FILE.into(),
                        format: gba_core::rom_data::BG_DESCRIPTORS_FORMAT.into(),
                        sha256: String::new(),
                        dependencies: vec![],
                        sources: vec![SourceRange {
                            offset: 0x100,
                            length: (bg_descriptors::BASE_RECORDS * bg_descriptors::RECORD_BYTES)
                                as u32,
                            sha256: bg_hash,
                            role: "synthetic-bg".into(),
                        }],
                    },
                    Resource {
                        id: BG_BASE_PALETTES_ID.into(),
                        path: BG_BASE_PALETTES_FILE.into(),
                        format: BG_BASE_PALETTES_FORMAT.into(),
                        sha256: String::new(),
                        dependencies: vec![BG_DESCRIPTORS_ID.into()],
                        sources: palette_ranges,
                    },
                    Resource {
                        id: BG_ALTERNATE_PALETTES_ID.into(), path: BG_ALTERNATE_PALETTES_FILE.into(),
                        format: BG_ALTERNATE_PALETTES_FORMAT.into(), sha256: String::new(),
                        dependencies: vec![BG_DESCRIPTORS_ID.into()], sources: alternate_ranges,
                    },
                    Resource {
                        id: BG_PACKET_PROGRAMS_ID.into(), path: BG_PACKET_PROGRAMS_FILE.into(),
                        format: BG_PACKET_PROGRAMS_FORMAT.into(), sha256: String::new(),
                        dependencies: vec![BG_DESCRIPTORS_ID.into()], sources: packet_ranges,
                    },
                ],
            },
            dimensions,
            background: Some(background),
            palettes: Some(bg_palettes::Catalog {
                decoded: decoded_palettes,
                sources: palette_sources,
            }),
            alternate_palettes: Some(alternate_palettes),
            packet_programs: Some(packet_programs),
            main_graphics: None,
            transfers: None,
            short_texts: None,
            text_catalogs: None,
            name_keyboard: None,
            obj_lists: None,
            obj_animations: None,
            obj_graphics: None,
            ui_palettes: None,
            ui_graphics: None,
            items: None,
            item_links: None,
            metadata16: None,
            metadata52: None,
            metadata4: None,
            metadata2: None,
            default48: None,
            constructor_alternates: None,
            metadata14: None,
            metadata12: None,
            global_pair_flags: None,
            evaluation_parameters: None,
            progression_rewards: None,
            progression_gates: None,
            progression_selection_gates: None,
            aggregate_flag_keys: None,
            trade_parameters: None,
            item_effect_parameters: None,
            audio_directory: None,
            audio_tones: None,
            audio_sequences: None,
            audio_driver: None,
            palette_effect_dispatch: None,
            palette_programs: None,
            blend_programs: None,
            task_descriptors: None,
            effect_event_directories: None,
            script_commands: None,
            script_archive: None,
            script_text: None,
            script_coordinates: None,
            ui_choice_lists: None,
            unit_obj_initial_pose: None,
            color_mode_palettes: None,
            unit_obj_equipment: None,
            unit_obj_normal_battle: None,
            unit_obj_alternate_battle: None,
            unit_obj_script_special_normal: None,
            unit_obj_script_special_alternate: None,
            unit_obj_script_override: None,
            unit_obj_script_registry_special: None,
            unit_obj_script_registry_generic: None,
            unit_obj_property_words: None,
            unit_obj_byte_property_words: None,
            unit_obj_ffff_reader_fields: None,
            slot_job_lookup: None,
            observed_task_parameters: None,
            constructor_obj_animation: None,
            task_cycle_deltas: None,
            word_task_obj_animations: None,
            world_nodes: None,
            world_roads: None,
            encounter_headers: None,
            encounter_unit_templates: None,
            encounter_coordinates: None,
            metadata28: None,
            records4: None,
            conditional_records8: None,
            mission_records70: None,
            selection_byte_tables: None,
            event_records12: None,
            parameter_records27: None,
            world_records38: None,
            indexed_graphics: None,
            ui_raw_transfers: None,
            dynamic_raw_fragments: None,
            conditional_four_byte_lookup: None,
            conditional_packed_lookup: None,
            conditional_descriptor_banks: None,
            conditional_descriptor_bank_zero: None,
            conditional_property_masks: None,
            compact_obj_animations: None,
            secondary_compact_obj_animations: None,
            tertiary_compact_obj_animations: None,
            quaternary_compact_obj_animations: None,
            quinary_compact_obj_animations: None,
            senary_compact_obj_animations: None,
            map_raster_palettes: None,
            auxiliary_scripts: None,
            font_bank: None,
        }
    }

    #[test]
    fn deterministic_publication_reuse_and_relocation() {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("first folder/data");
        let b = temp.path().join("second");
        let mut source = fixture();
        assert_eq!(extract(&a, &mut source).unwrap(), "created");
        let before = fs::read(a.join(MANIFEST_FILE)).unwrap();
        assert_eq!(extract(&a, &mut source).unwrap(), "reused");
        assert_eq!(before, fs::read(a.join(MANIFEST_FILE)).unwrap());
        assert_eq!(extract(&b, &mut source).unwrap(), "created");
        for name in [
            MANIFEST_FILE,
            OBJ_DIMENSIONS_FILE,
            BG_DESCRIPTORS_FILE,
            BG_BASE_PALETTES_FILE,
            BG_ALTERNATE_PALETTES_FILE,
            BG_PACKET_PROGRAMS_FILE,
        ] {
            assert_eq!(
                fs::read(a.join(name)).unwrap(),
                fs::read(b.join(name)).unwrap()
            );
        }
        let moved = temp.path().join("moved folder");
        fs::rename(&a, &moved).unwrap();
        assert_eq!(verify(&moved, &source).unwrap(), source.dimensions);
        assert!(!String::from_utf8(before)
            .unwrap()
            .contains(&temp.path().display().to_string()));
    }

    #[test]
    fn corruption_missing_files_and_edits_fail_without_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let mut source = fixture();
        extract(&path, &mut source).unwrap();
        let original = fs::read(path.join(OBJ_DIMENSIONS_FILE)).unwrap();
        fs::write(path.join(OBJ_DIMENSIONS_FILE), b"broken").unwrap();
        assert!(verify(&path, &source).is_err());
        assert!(extract(&path, &mut source).is_err());
        assert_eq!(fs::read(path.join(OBJ_DIMENSIONS_FILE)).unwrap(), b"broken");
        fs::write(path.join(OBJ_DIMENSIONS_FILE), &original).unwrap();
        let mut changed = source.dimensions.clone();
        changed.entries[0].width = 64;
        let bytes = toml::to_string_pretty(&changed).unwrap();
        let mut manifest = source.manifest.clone();
        manifest.resources[0].sha256 = gba_core::ffta_rom::digest(bytes.as_bytes());
        fs::write(path.join(OBJ_DIMENSIONS_FILE), &bytes).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify(&path, &source).is_err());
        fs::remove_file(path.join(OBJ_DIMENSIONS_FILE)).unwrap();
        assert!(verify(&path, &source).is_err());
        fs::write(
            path.join(OBJ_DIMENSIONS_FILE),
            [0; MAX_FILE_BYTES as usize + 1],
        )
        .unwrap();
        assert!(verify(&path, &source).is_err());
    }

    #[test]
    fn strict_parser_and_fixed_paths() {
        let source = fixture();
        let bytes = toml::to_string_pretty(&source.dimensions).unwrap();
        let wrong = format!("unknown = 1\n{bytes}");
        assert!(parse::<ObjDimensions>(wrong.as_bytes(), "test").is_err());
        let wrong = bytes.replacen("width = 8", "width = 65536", 1);
        assert!(parse::<ObjDimensions>(wrong.as_bytes(), "test").is_err());
        assert!(parse::<ObjDimensions>(&[255], "test").is_err());
        let fewer = ObjDimensions {
            entries: source.dimensions.entries,
        };
        let text = toml::to_string_pretty(&fewer).unwrap();
        let cut = text.rfind("[[entries]]").unwrap();
        assert!(parse::<ObjDimensions>(text[..cut].as_bytes(), "test").is_err());
        assert!(cmd(&[]).is_err());
        assert!(cmd(&["unknown".into(), "rom".into(), "out".into()]).is_err());
    }

    #[test]
    fn background_json_is_direct_strict_and_deterministic() {
        let source = fixture();
        let background = source.background.as_ref().unwrap();
        let bytes = encode_background(background).unwrap();
        assert_eq!(bytes, encode_background(background).unwrap());
        assert_eq!(&parse_background(&bytes).unwrap(), background);
        assert!(bytes.starts_with(b"{\n  \"entries\": ["));
        assert!(bytes.len() as u64 <= MAX_BG_DESCRIPTORS_FILE_BYTES);
        assert!(parse_background(&[255]).is_err());
        assert!(parse_background(&bytes[..bytes.len() - 1]).is_err());
        let mut trailing = bytes.clone();
        trailing.extend_from_slice(b" true");
        assert!(parse_background(&trailing).is_err());
        assert!(parse_background(br#"{"entries":[],"entries":[]}"#).is_err());
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for invalid in 0..6 {
            let mut changed = value.clone();
            match invalid {
                0 => changed["entries"][0]["unknown"] = serde_json::json!(0),
                1 => changed["entries"][0]["graphics_source"] = serde_json::json!(4294967296u64),
                2 => changed["entries"][0]["selectors"][0] = serde_json::json!(256),
                3 => {
                    changed["entries"][0]
                        .as_object_mut()
                        .unwrap()
                        .remove("selectors");
                }
                4 => {
                    changed["entries"][0]["extra_transfers"]
                        .as_array_mut()
                        .unwrap()
                        .pop();
                }
                5 => changed["entries"] = serde_json::json!([]),
                _ => unreachable!(),
            }
            assert!(parse_background(&serde_json::to_vec(&changed).unwrap()).is_err());
        }
    }

    #[test]
    fn background_is_required_and_rehashed_edits_are_not_base_data() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let mut source = fixture();
        extract(&path, &mut source).unwrap();
        let original = fs::read(path.join(BG_DESCRIPTORS_FILE)).unwrap();
        assert!(original.len() > MAX_FILE_BYTES as usize);
        let mut edited = source.background.clone().unwrap();
        edited.entries[0].selectors[0] ^= 1;
        let bytes = encode_background(&edited).unwrap();
        let mut manifest = source.manifest.clone();
        manifest.resources[1].sha256 = gba_core::ffta_rom::digest(&bytes);
        fs::write(path.join(BG_DESCRIPTORS_FILE), &bytes).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify(&path, &source).unwrap_err().contains("reproduce"));
        assert!(extract(&path, &mut source).is_err());
        assert_eq!(
            fs::read(path.join(BG_DESCRIPTORS_FILE)).unwrap(),
            bytes.as_slice()
        );
        fs::write(path.join(BG_DESCRIPTORS_FILE), original).unwrap();
        for change in 0..4 {
            let mut bad = source.manifest.clone();
            match change {
                0 => {
                    bad.resources.remove(1);
                }
                1 => bad.resources.push(bad.resources[1].clone()),
                2 => bad.resources[1].path = "../outside".into(),
                _ => bad.resources[1].sources[0].offset += 1,
            }
            fs::write(
                path.join(MANIFEST_FILE),
                toml::to_string_pretty(&bad).unwrap(),
            )
            .unwrap();
            assert!(verify(&path, &source).is_err());
        }
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&source.manifest).unwrap(),
        )
        .unwrap();
        fs::remove_file(path.join(BG_DESCRIPTORS_FILE)).unwrap();
        assert!(verify(&path, &source).is_err());
    }

    #[test]
    fn palette_json_preserves_words_and_rejects_malformed_or_forged_data() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let mut source = fixture();
        extract(&path, &mut source).unwrap();
        let bytes = fs::read(path.join(BG_BASE_PALETTES_FILE)).unwrap();
        let palettes = parse_palettes(&bytes).unwrap();
        assert_eq!(encode_palettes(&palettes).unwrap(), bytes);
        assert_eq!(palettes.entries[0].banks[0][0], 0x8123);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for invalid in 0..6 {
            let mut changed = value.clone();
            match invalid {
                0 => changed["entries"][0]["unknown"] = serde_json::json!(0),
                1 => changed["entries"][0]["banks"][0][0] = serde_json::json!(65536),
                2 => {
                    changed["entries"][0]["banks"][0]
                        .as_array_mut()
                        .unwrap()
                        .pop();
                }
                3 => changed["entries"][0]["banks"] = serde_json::json!([]),
                4 => changed["entries"] = serde_json::json!([]),
                5 => changed["entries"][0]["banks"][0][0] = serde_json::json!(1),
                _ => unreachable!(),
            }
            assert!(parse_palettes(&serde_json::to_vec(&changed).unwrap()).is_err());
        }
        assert!(parse_palettes(&bytes[..bytes.len() - 1]).is_err());
        // Equal decoded words with different source bytes would change guest
        // branches/reads, so even a coherent re-encoding is not valid base data.
        let mut recoded = palettes.clone();
        let words = recoded.entries[0].source_bytes().unwrap();
        let mut encoded = (((words.len() as u32) << 8) | 0x10).to_le_bytes().to_vec();
        for group in words.chunks(8) {
            encoded.push(0);
            encoded.extend_from_slice(group);
        }
        recoded.entries[0].encoded_source = encoded;
        let recoded = encode_palettes(&recoded).unwrap();
        let mut manifest = source.manifest.clone();
        manifest.resources[2].sha256 = gba_core::ffta_rom::digest(&recoded);
        fs::write(path.join(BG_BASE_PALETTES_FILE), recoded).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify(&path, &source).unwrap_err().contains("reproduce"));
        let mut edited = palettes;
        edited.entries[0].banks[0][0] ^= 0x8000;
        let word = edited.entries[0].banks[0][0].to_le_bytes();
        edited.entries[0].encoded_source[4..6].copy_from_slice(&word);
        let edited = encode_palettes(&edited).unwrap();
        let mut manifest = source.manifest.clone();
        manifest.resources[2].sha256 = gba_core::ffta_rom::digest(&edited);
        fs::write(path.join(BG_BASE_PALETTES_FILE), edited).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify(&path, &source).unwrap_err().contains("reproduce"));
        fs::write(path.join(BG_BASE_PALETTES_FILE), bytes).unwrap();
        for invalid in 0..4 {
            let mut manifest = source.manifest.clone();
            match invalid {
                0 => {
                    manifest.resources.remove(2);
                }
                1 => manifest.resources[2].path = "../outside".into(),
                2 => manifest.resources[2].dependencies.clear(),
                3 => manifest.resources[2].sources[0].offset += 4,
                _ => unreachable!(),
            }
            fs::write(
                path.join(MANIFEST_FILE),
                toml::to_string_pretty(&manifest).unwrap(),
            )
            .unwrap();
            assert!(verify(&path, &source).is_err());
        }
        fs::remove_file(path.join(BG_BASE_PALETTES_FILE)).unwrap();
        assert!(verify(&path, &source).is_err());
    }

    #[test]
    fn per_family_input_bounds_and_complete_set_limit() {
        fn padded(mut bytes: Vec<u8>, length: usize) -> Vec<u8> {
            bytes.resize(length, b' ');
            bytes
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let mut source = fixture();
        extract(&path, &mut source).unwrap();
        let names = [
            MANIFEST_FILE,
            OBJ_DIMENSIONS_FILE,
            BG_DESCRIPTORS_FILE,
            BG_BASE_PALETTES_FILE,
            BG_ALTERNATE_PALETTES_FILE,
            BG_PACKET_PROGRAMS_FILE,
        ];
        let original: Vec<_> = names
            .iter()
            .map(|name| fs::read(path.join(name)).unwrap())
            .collect();
        let bg = padded(original[2].clone(), MAX_BG_DESCRIPTORS_FILE_BYTES as usize);
        let palettes = padded(
            original[3].clone(),
            MAX_BG_BASE_PALETTES_FILE_BYTES as usize,
        );
        let mut manifest = source.manifest.clone();
        manifest.resources[1].sha256 = gba_core::ffta_rom::digest(&bg);
        fs::write(path.join(BG_DESCRIPTORS_FILE), &bg).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        verify(&path, &source).unwrap();
        manifest.resources[2].sha256 = gba_core::ffta_rom::digest(&palettes);
        fs::write(path.join(BG_BASE_PALETTES_FILE), &palettes).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        verify(&path, &source).unwrap();
        let alternates = padded(original[4].clone(), MAX_BG_ALTERNATE_PALETTES_FILE_BYTES as usize);
        manifest.resources[3].sha256 = gba_core::ffta_rom::digest(&alternates);
        fs::write(path.join(BG_ALTERNATE_PALETTES_FILE), &alternates).unwrap();
        fs::write(path.join(MANIFEST_FILE), toml::to_string_pretty(&manifest).unwrap()).unwrap();
        let packets = padded(original[5].clone(), MAX_BG_PACKET_PROGRAMS_FILE_BYTES as usize);
        manifest.resources[4].sha256 = gba_core::ffta_rom::digest(&packets);
        fs::write(path.join(BG_PACKET_PROGRAMS_FILE), &packets).unwrap();
        fs::write(path.join(MANIFEST_FILE), toml::to_string_pretty(&manifest).unwrap()).unwrap();
        // The older six-file fixture now fits inside the complete-set limit.
        verify(&path, &source).unwrap();
        // Exercise the actual aggregate boundary without allocating that much
        // JSON. Each individual file still fits its own input guard.
        check_dataset_budget(0, 0, 0, 0, 0, 0, 0, 0, 0, MAX_DATASET_BYTES, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0).unwrap();
        assert!(check_dataset_budget(0, 1, 0, 0, 0, 0, 0, 0, 0, MAX_DATASET_BYTES, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
            .unwrap_err()
            .contains("total size"));
        for (name, limit) in [
            (MANIFEST_FILE, MAX_MANIFEST_FILE_BYTES),
            (OBJ_DIMENSIONS_FILE, MAX_FILE_BYTES),
            (BG_DESCRIPTORS_FILE, MAX_BG_DESCRIPTORS_FILE_BYTES),
            (BG_BASE_PALETTES_FILE, MAX_BG_BASE_PALETTES_FILE_BYTES),
            (BG_ALTERNATE_PALETTES_FILE, MAX_BG_ALTERNATE_PALETTES_FILE_BYTES),
            (BG_PACKET_PROGRAMS_FILE, MAX_BG_PACKET_PROGRAMS_FILE_BYTES),
        ] {
            for (name, bytes) in names.iter().zip(&original) {
                fs::write(path.join(name), bytes).unwrap();
            }
            fs::File::options()
                .write(true)
                .open(path.join(name))
                .unwrap()
                .set_len(limit + 1)
                .unwrap();
            assert!(verify(&path, &source)
                .unwrap_err()
                .contains(&format!("at most {limit}")));
        }
    }

    #[test]
    fn alternate_json_is_strict_and_rehashed_edits_are_not_base_data() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let mut source = fixture();
        extract(&path, &mut source).unwrap();
        let original = fs::read(path.join(BG_ALTERNATE_PALETTES_FILE)).unwrap();
        let parsed = parse_alternate_palettes(&original).unwrap();
        assert_eq!(encode_alternate_palettes(&parsed).unwrap(), original);
        let value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        for invalid in 0..8 {
            let mut changed = value.clone();
            match invalid {
                0 => changed["unknown"] = serde_json::json!(0),
                1 => changed["catalogs"][0][0]["banks"][0][0] = serde_json::json!(65536),
                2 => { changed["catalogs"][0][0]["banks"][0].as_array_mut().unwrap().pop(); },
                3 => changed["catalogs"][0][0]["encoded_source"] = serde_json::json!([0,0,1,0,0x80]),
                4 => changed["descriptor_keys"][0] = serde_json::json!(157),
                5 => changed["relative_sources"][0][0] = serde_json::json!(0),
                6 => changed["catalogs"] = serde_json::json!([]),
                _ => changed["descriptor_keys"][0] = serde_json::json!(255),
            }
            assert!(parse_alternate_palettes(&serde_json::to_vec(&changed).unwrap()).is_err());
        }
        assert!(parse_alternate_palettes(&original[..original.len()-1]).is_err());
        // A new canonical source representation with equal banks is still an
        // edit to original guest reads and cannot enter as authenticated base.
        let mut recoded = parsed;
        recoded.catalogs[0][0].encoded_source.insert(4, 3);
        let recoded = encode_alternate_palettes(&recoded).unwrap();
        let mut manifest = source.manifest.clone();
        manifest.resources[3].sha256 = gba_core::ffta_rom::digest(&recoded);
        fs::write(path.join(BG_ALTERNATE_PALETTES_FILE), recoded).unwrap();
        fs::write(path.join(MANIFEST_FILE), toml::to_string_pretty(&manifest).unwrap()).unwrap();
        assert!(verify(&path, &source).unwrap_err().contains("reproduce"));
        fs::write(path.join(BG_ALTERNATE_PALETTES_FILE), original).unwrap();
        for invalid in 0..4 {
            let mut manifest = source.manifest.clone();
            match invalid {
                0 => manifest.resources[3].path = "../outside".into(),
                1 => manifest.resources[3].dependencies.clear(),
                2 => manifest.resources[3].sources[0].offset += 2,
                _ => { manifest.resources.pop(); },
            }
            fs::write(path.join(MANIFEST_FILE), toml::to_string_pretty(&manifest).unwrap()).unwrap();
            assert!(verify(&path, &source).is_err());
        }
    }

    #[test]
    fn competing_publishers_leave_one_verified_dataset() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    extract(&path, &mut fixture()).unwrap()
                })
            })
            .collect();
        let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(outcomes.iter().filter(|&&v| v == "created").count(), 1);
        verify(&path, &fixture()).unwrap();
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    #[ignore = "requires the user's exact USA and EUR ROMs; no game data is bundled"]
    fn regional_cli_extraction_verification_and_identity() {
        let temp = tempfile::tempdir().unwrap();
        let mut decoded = Vec::new();
        let mut manifests = Vec::new();
        for (env, profile) in [
            ("FFTA_TEST_ROM_USA", "usa"),
            ("FFTA_TEST_ROM_EUR", "europe"),
        ] {
            let rom_path = std::env::var(env).unwrap();
            let output = temp.path().join(profile);
            let output_arg = output.to_str().unwrap().to_owned();
            cmd(&["extract".into(), rom_path.clone(), output_arg.clone()]).unwrap();
            let manifest = fs::read(output.join(MANIFEST_FILE)).unwrap();
            cmd(&["extract".into(), rom_path.clone(), output_arg.clone()]).unwrap();
            assert_eq!(manifest, fs::read(output.join(MANIFEST_FILE)).unwrap());
            cmd(&["verify".into(), rom_path.clone(), output_arg]).unwrap();
            let source = Extraction::from_rom(&fs::read(&rom_path).unwrap()).unwrap();
            assert_eq!(verify(&output, &source).unwrap(), source.dimensions);
            decoded.push(fs::read(output.join(OBJ_DIMENSIONS_FILE)).unwrap());
            manifests.push(manifest);
        }
        assert_eq!(decoded[0], decoded[1]);
        assert_ne!(manifests[0], manifests[1]);
        let usa = fs::read(std::env::var("FFTA_TEST_ROM_USA").unwrap()).unwrap();
        assert!(verify(
            &temp.path().join("europe"),
            &Extraction::from_rom(&usa).unwrap()
        )
        .is_err());
    }

    #[test]
    #[ignore = "requires the user's exact USA and EUR ROMs; no game data is bundled"]
    fn runtime_files_bind_transactionally_and_survive_slots_undo_and_backup_reboot() {
        use gba_core::Bus;
        use std::sync::Arc;
        for env in ["FFTA_TEST_ROM_USA", "FFTA_TEST_ROM_EUR"] {
            let temp = tempfile::tempdir().unwrap();
            let rom = fs::read(std::env::var(env).unwrap()).unwrap();
            let mut source = Extraction::from_rom(&rom).unwrap();
            let path = temp.path().join("decoded");
            extract(&path, &mut source).unwrap();
            let mut machine = super::super::make_machine(rom, None);
            let boot = machine.save_state().unwrap();
            attach_from_directory(&mut machine, &path).unwrap();
            let resources = machine.bus.rom_resources().unwrap().clone();
            assert_eq!(
                resources.file_sha256(),
                gba_core::ffta_rom::digest(&fs::read(path.join(OBJ_DIMENSIONS_FILE)).unwrap())
            );
            assert_eq!(boot, machine.save_state().unwrap());
            let offset = source.manifest.resources[0].sources[0].offset;
            let expected = machine.bus.read32(0x08000000 + offset);
            let save = temp.path().join("saves/test.sav");
            fs::create_dir_all(save.parent().unwrap()).unwrap();
            let mut slots = super::super::save_state::Slots::new(&save, &machine.bus.rom);
            let preview = vec![[0; 4]; 240 * 160];
            slots.save(&machine, 1, &preview, false).unwrap();
            machine.cpu.regs[4] = 37;
            let before = machine.save_state().unwrap();
            slots.load(&mut machine, 1).unwrap();
            assert_eq!(machine.cpu.regs[4], 0);
            assert!(Arc::ptr_eq(
                machine.bus.rom_resources().unwrap(),
                &resources
            ));
            slots.undo(&mut machine).unwrap();
            assert_eq!(machine.save_state().unwrap(), before);
            assert!(Arc::ptr_eq(
                machine.bus.rom_resources().unwrap(),
                &resources
            ));
            // A new activation must reject absent files; a running session
            // retains its already verified immutable in-memory resources.
            fs::remove_file(path.join(OBJ_DIMENSIONS_FILE)).unwrap();
            assert!(attach_from_directory(&mut machine, &path).is_err());
            assert!(Arc::ptr_eq(
                machine.bus.rom_resources().unwrap(),
                &resources
            ));
            let mut fresh = super::super::make_machine(machine.bus.rom.clone(), None);
            assert!(attach_from_directory(&mut fresh, &path).is_err());
            assert!(fresh.bus.rom_resources().is_none());
            assert_eq!(machine.bus.read32(0x08000000 + offset), expected);
            let count = resources.stats();
            machine.load_state(&boot).unwrap();
            assert!(Arc::ptr_eq(
                machine.bus.rom_resources().unwrap(),
                &resources
            ));
            assert_eq!(resources.stats(), count);
            let length = machine.bus.save_data().unwrap().len();
            let backups = super::super::save_backups::Backups::new(&save);
            fs::write(&save, vec![7; length]).unwrap();
            backups.capture_startup().unwrap();
            let generation = backups.choices(length)[0].generation.unwrap();
            machine.cpu.regs[4] = 99;
            backups.restore(generation, &mut machine, &boot).unwrap();
            assert_eq!(machine.cpu.regs[4], 0);
            assert!(Arc::ptr_eq(
                machine.bus.rom_resources().unwrap(),
                &resources
            ));
            assert_eq!(machine.bus.save_data().unwrap(), vec![7; length]);
            assert_eq!(machine.bus.read32(0x08000000 + offset), expected);
            assert!(resources.stats().bus_reads > 0);
        }
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_resources_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real");
        let alias = temp.path().join("alias");
        let mut source = fixture();
        extract(&real, &mut source).unwrap();
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        assert!(verify(&alias, &source).is_err());
        fs::rename(real.join(OBJ_DIMENSIONS_FILE), temp.path().join("outside")).unwrap();
        std::os::unix::fs::symlink(temp.path().join("outside"), real.join(OBJ_DIMENSIONS_FILE))
            .unwrap();
        assert!(verify(&real, &source).is_err());
        fs::remove_file(real.join(OBJ_DIMENSIONS_FILE)).unwrap();
        fs::rename(temp.path().join("outside"), real.join(OBJ_DIMENSIONS_FILE)).unwrap();
        fs::rename(
            real.join(BG_DESCRIPTORS_FILE),
            temp.path().join("outside-bg"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            temp.path().join("outside-bg"),
            real.join(BG_DESCRIPTORS_FILE),
        )
        .unwrap();
        assert!(verify(&real, &source).is_err());
    }
}
