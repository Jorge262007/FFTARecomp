//! Versioned decoded base resources. No filesystem I/O or guest state.
//! Only exact, independently verified profiles are eligible for extraction.
use crate::ffta_rom::{self, Version};
use serde::{Deserialize, Serialize};

mod runtime;
#[cfg(feature = "rom-data-read-audit")]
pub mod read_audit;
pub use runtime::{AlternatePaletteStats, BackgroundStats, GraphicsStats, PacketRead, PacketStats, PaletteRead, PaletteStats, ResourceStats, RuntimeResources, TransferStats, TextStats, TextCatalogStats, FontKeyboardStats, ObjListStats, ObjAnimationStats};
pub use runtime::{CompactObjStats, SecondaryCompactObjStats, TertiaryCompactObjStats};
pub mod bg_descriptors;
pub mod bg_packets;
pub mod bg_packet_data;
pub mod bg_palettes;
pub mod bg_alternate_palettes;
pub mod bg_graphics;
pub mod bg_transfers;
pub mod text_short;
pub mod text_catalogs;
pub mod fonts;
pub mod name_keyboard;
pub mod obj_lists;
pub mod obj_animations;
pub mod compact_obj_animations;
pub mod compact_obj_animations_secondary;
pub mod compact_obj_animations_tertiary;
pub mod compact_obj_animations_quaternary;
pub mod compact_obj_animations_quinary;
pub mod compact_obj_animations_senary;
pub mod word_task_obj_animations;
pub mod obj_graphics;
pub mod ui_palettes;
pub mod ui_graphics;
pub mod ui_raw_transfers;
pub mod items;
pub mod item_links;
pub mod metadata16;
pub mod metadata52;
pub mod metadata4;
pub mod metadata2;
pub mod metadata28;
pub mod records4;
pub mod conditional_records8;
pub mod mission_records70;
pub mod selection_byte_tables;
pub mod event_records12;
pub mod parameter_records27;
pub mod world_records38;
pub mod indexed_graphics;
pub mod default48;
pub mod constructor_alternates;
pub mod metadata14;
pub mod metadata12;
pub mod global_pair_flags;
pub mod evaluation_parameters;
pub mod conditional_four_byte_lookup;
pub mod conditional_packed_lookup;
pub mod conditional_descriptor_banks;
pub mod conditional_descriptor_bank_zero;
pub mod conditional_property_masks;
pub mod progression_rewards;
pub mod progression_gates;
pub mod progression_selection_gates;
pub mod aggregate_flag_keys;
pub mod trade_parameters;
pub mod item_effect_parameters;
pub mod audio_directory;
mod audio_producers;
pub mod audio_tones;
pub mod audio_sequences;
pub mod audio_driver;
pub mod palette_effect_dispatch;
pub mod palette_programs;
pub mod blend_programs;
pub mod task_descriptors;
pub mod effect_event_directories;
pub mod script_commands;
pub mod script_archive;
pub mod script_text;
pub mod script_coordinates;
pub mod auxiliary_scripts;
pub mod map_raster_palettes;
pub mod ui_choice_lists;
pub mod unit_obj_initial_pose;
pub mod unit_obj_equipment;
pub mod unit_obj_normal_battle;
pub mod unit_obj_script_special_normal;
pub mod unit_obj_script_special_alternate;
pub mod unit_obj_script_override;
pub mod unit_obj_script_registry_special;
pub mod unit_obj_script_registry_generic;
pub mod unit_obj_property_words;
pub mod unit_obj_ffff_reader_fields;
pub mod slot_job_lookup;
pub mod observed_task_parameters;
pub mod task_cycle_deltas;
pub mod world_nodes;
pub mod world_roads;
pub mod encounter_headers;
pub mod encounter_unit_templates;
pub mod encounter_coordinates;
pub mod unit_obj_byte_property_words;
pub mod unit_obj_alternate_battle;
pub mod color_mode_palettes;
mod custom_lz;
pub mod lz77;

pub const SCHEMA: u32 = 1;
pub const EXTRACTOR_REV: u32 = 106;
pub const DECODER_REV: u32 = 105;
pub const METADATA16_ID: &str = "gameplay.linked-metadata16";
pub const METADATA16_FILE: &str = "linked-metadata16.json";
pub const METADATA16_FORMAT: &str = "linked-metadata16-v1-json";
pub const MAX_METADATA16_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const METADATA52_ID: &str = "gameplay.linked-metadata52";
pub const METADATA52_FILE: &str = "linked-metadata52.json";
pub const METADATA52_FORMAT: &str = "linked-metadata52-v1-json";
pub const MAX_METADATA52_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const METADATA4_ID: &str = "gameplay.linked-metadata4";
pub const METADATA4_FILE: &str = "linked-metadata4.json";
pub const METADATA4_FORMAT: &str = "linked-metadata4-v1-json";
pub const MAX_METADATA4_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const METADATA2_ID: &str = "gameplay.linked-metadata2";
pub const METADATA2_FILE: &str = "linked-metadata2.json";
pub const METADATA2_FORMAT: &str = "linked-metadata2-v1-json";
pub const MAX_METADATA2_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const DEFAULT48_ID: &str = "gameplay.default-constructor-template";
pub const DEFAULT48_FILE: &str = "default-constructor-template.json";
pub const DEFAULT48_FORMAT: &str = "default-constructor-template-v1-json";
pub const MAX_DEFAULT48_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const CONSTRUCTOR_ALTERNATES_ID: &str = "gameplay.constructor-alternate-templates";
pub const CONSTRUCTOR_ALTERNATES_FILE: &str = "constructor-alternate-templates.json";
pub const CONSTRUCTOR_ALTERNATES_FORMAT: &str = "constructor-alternate-templates-v1-json";
pub const MAX_CONSTRUCTOR_ALTERNATES_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const METADATA14_ID: &str = "gameplay.linked-metadata14";
pub const METADATA14_FILE: &str = "linked-metadata14.json";
pub const METADATA14_FORMAT: &str = "linked-metadata14-v1-json";
pub const MAX_METADATA14_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const METADATA12_ID: &str = "gameplay.linked-metadata12";
pub const METADATA12_FILE: &str = "linked-metadata12.json";
pub const METADATA12_FORMAT: &str = "linked-metadata12-v1-json";
pub const MAX_METADATA12_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const GLOBAL_PAIR_FLAGS_ID: &str = "gameplay.global-pair-flags";
pub const GLOBAL_PAIR_FLAGS_FILE: &str = "global-pair-flags.json";
pub const GLOBAL_PAIR_FLAGS_FORMAT: &str = "global-pair-flags-v1-json";
pub const MAX_GLOBAL_PAIR_FLAGS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const EVALUATION_PARAMETERS_ID: &str = "gameplay.evaluation-parameters";
pub const EVALUATION_PARAMETERS_FILE: &str = "evaluation-parameters.json";
pub const EVALUATION_PARAMETERS_FORMAT: &str = "evaluation-parameters-v1-json";
pub const MAX_EVALUATION_PARAMETERS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const PROGRESSION_REWARDS_ID: &str = "gameplay.progression-rewards";
pub const PROGRESSION_REWARDS_FILE: &str = "progression-rewards.json";
pub const PROGRESSION_REWARDS_FORMAT: &str = "progression-rewards-v1-json";
pub const MAX_PROGRESSION_REWARDS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const PROGRESSION_GATES_ID: &str = "gameplay.progression-gates";
pub const PROGRESSION_GATES_FILE: &str = "progression-gates.json";
pub const PROGRESSION_GATES_FORMAT: &str = "progression-gates-v1-json";
pub const MAX_PROGRESSION_GATES_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const PROGRESSION_SELECTION_GATES_ID: &str = "gameplay.progression-selection-gates";
pub const PROGRESSION_SELECTION_GATES_FILE: &str = "progression-selection-gates.json";
pub const PROGRESSION_SELECTION_GATES_FORMAT: &str = "progression-selection-gates-v1-json";
pub const MAX_PROGRESSION_SELECTION_GATES_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const AGGREGATE_FLAG_KEYS_ID: &str = "gameplay.aggregate-flag-keys";
pub const AGGREGATE_FLAG_KEYS_FILE: &str = "aggregate-flag-keys.json";
pub const AGGREGATE_FLAG_KEYS_FORMAT: &str = "aggregate-flag-keys-v1-json";
pub const MAX_AGGREGATE_FLAG_KEYS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const TRADE_PARAMETERS_ID: &str = "gameplay.trade-parameters";
pub const TRADE_PARAMETERS_FILE: &str = "trade-parameters.json";
pub const TRADE_PARAMETERS_FORMAT: &str = "trade-parameters-v1-json";
pub const MAX_TRADE_PARAMETERS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const ITEM_EFFECT_PARAMETERS_ID: &str = "gameplay.item-effect-parameters";
pub const ITEM_EFFECT_PARAMETERS_FILE: &str = "item-effect-parameters.json";
pub const ITEM_EFFECT_PARAMETERS_FORMAT: &str = "item-effect-parameters-v1-json";
pub const MAX_ITEM_EFFECT_PARAMETERS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const AUDIO_DIRECTORY_ID: &str = "audio.m4a-directory";
pub const AUDIO_DIRECTORY_FILE: &str = "audio-directory.json";
pub const AUDIO_DIRECTORY_FORMAT: &str = "audio-directory-v1-json";
pub const MAX_AUDIO_DIRECTORY_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const AUDIO_TONES_ID: &str = "audio.m4a-tones";
pub const AUDIO_TONES_FILE: &str = "audio-tones.json";
pub const AUDIO_TONES_FORMAT: &str = "audio-tones-v1-json";
pub const MAX_AUDIO_TONES_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const AUDIO_SEQUENCES_ID: &str = "audio.m4a-sequences";
pub const AUDIO_SEQUENCES_FILE: &str = "audio-sequences.json";
pub const AUDIO_SEQUENCES_FORMAT: &str = "audio-sequences-v1-json";
pub const MAX_AUDIO_SEQUENCES_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const AUDIO_DRIVER_ID: &str = "audio.m4a-driver-tables";
pub const AUDIO_DRIVER_FILE: &str = "audio-driver-tables.json";
pub const AUDIO_DRIVER_FORMAT: &str = "audio-driver-tables-v1-json";
pub const MAX_AUDIO_DRIVER_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const PALETTE_EFFECT_DISPATCH_ID: &str = "graphics.palette-effect-dispatch";
pub const PALETTE_EFFECT_DISPATCH_FILE: &str = "palette-effect-dispatch.json";
pub const PALETTE_EFFECT_DISPATCH_FORMAT: &str = "palette-effect-dispatch-v1-json";
pub const MAX_PALETTE_EFFECT_DISPATCH_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const PALETTE_PROGRAMS_ID: &str = "graphics.palette-programs";
pub const PALETTE_PROGRAMS_FILE: &str = "palette-programs.json";
pub const PALETTE_PROGRAMS_FORMAT: &str = "palette-programs-v1-json";
pub const MAX_PALETTE_PROGRAMS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const BLEND_PROGRAMS_ID: &str = "graphics.blend-programs";
pub const BLEND_PROGRAMS_FILE: &str = "blend-programs.json";
pub const BLEND_PROGRAMS_FORMAT: &str = "blend-programs-v1-json";
pub const MAX_BLEND_PROGRAMS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const TASK_DESCRIPTORS_ID: &str = "engine.task-descriptors";
pub const TASK_DESCRIPTORS_FILE: &str = "task-descriptors.json";
pub const TASK_DESCRIPTORS_FORMAT: &str = "task-descriptors-v1-json";
pub const MAX_TASK_DESCRIPTORS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const EFFECT_EVENT_DIRECTORIES_ID: &str = "engine.effect-event-directories";
pub const EFFECT_EVENT_DIRECTORIES_FILE: &str = "effect-event-directories.json";
pub const EFFECT_EVENT_DIRECTORIES_FORMAT: &str = "effect-event-directories-v1-json";
pub const MAX_EFFECT_EVENT_DIRECTORIES_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const SCRIPT_COMMANDS_ID: &str = "engine.script-command-directory";
pub const SCRIPT_COMMANDS_FILE: &str = "script-command-directory.json";
pub const SCRIPT_COMMANDS_FORMAT: &str = "script-command-directory-v1-json";
pub const MAX_SCRIPT_COMMANDS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const SCRIPT_ARCHIVE_ID: &str = "engine.script-archive";
pub const SCRIPT_ARCHIVE_FILE: &str = "script-archive.json";
pub const SCRIPT_ARCHIVE_FORMAT: &str = "script-archive-v1-json";
pub const MAX_SCRIPT_ARCHIVE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const SCRIPT_TEXT_ID: &str = "text.script-archives";
pub const SCRIPT_TEXT_FILE: &str = "script-text.json";
pub const SCRIPT_TEXT_FORMAT: &str = "script-text-v1-json";
pub const MAX_SCRIPT_TEXT_FILE_BYTES: u64 = 512 * 1024 * 1024;
pub const SCRIPT_COORDINATES_ID: &str = "engine.script-coordinate-paths";
pub const SCRIPT_COORDINATES_FILE: &str = "script-coordinate-paths.json";
pub const SCRIPT_COORDINATES_FORMAT: &str = "script-coordinate-paths-v1-json";
pub const MAX_SCRIPT_COORDINATES_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UI_CHOICE_LISTS_ID: &str = "engine.ui-choice-lists";
pub const UI_CHOICE_LISTS_FILE: &str = "ui-choice-lists.json";
pub const UI_CHOICE_LISTS_FORMAT: &str = "ui-choice-lists-v1-json";
pub const MAX_UI_CHOICE_LISTS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_INITIAL_POSE_ID: &str = "graphics.unit-obj-initial-pose";
pub const UNIT_OBJ_INITIAL_POSE_FILE: &str = "unit-obj-initial-pose.json";
pub const UNIT_OBJ_INITIAL_POSE_FORMAT: &str = "unit-obj-initial-pose-v1-json";
pub const MAX_UNIT_OBJ_INITIAL_POSE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const COLOR_MODE_PALETTES_ID: &str = "graphics.color-mode-palettes";
pub const COLOR_MODE_PALETTES_FILE: &str = "color-mode-palettes.json";
pub const COLOR_MODE_PALETTES_FORMAT: &str = "color-mode-palettes-v1-json";
pub const MAX_COLOR_MODE_PALETTES_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_EQUIPMENT_ID: &str = "graphics.unit-obj-equipment";
pub const UNIT_OBJ_EQUIPMENT_FILE: &str = "unit-obj-equipment.json";
pub const UNIT_OBJ_EQUIPMENT_FORMAT: &str = "unit-obj-equipment-v1-json";
pub const MAX_UNIT_OBJ_EQUIPMENT_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_NORMAL_BATTLE_ID: &str = "graphics.unit-obj-normal-battle";
pub const UNIT_OBJ_NORMAL_BATTLE_FILE: &str = "unit-obj-normal-battle.json";
pub const UNIT_OBJ_NORMAL_BATTLE_FORMAT: &str = "unit-obj-normal-battle-v1-json";
pub const MAX_UNIT_OBJ_NORMAL_BATTLE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_ALTERNATE_BATTLE_ID: &str = "graphics.unit-obj-alternate-battle";
pub const UNIT_OBJ_ALTERNATE_BATTLE_FILE: &str = "unit-obj-alternate-battle.json";
pub const UNIT_OBJ_ALTERNATE_BATTLE_FORMAT: &str = "unit-obj-alternate-battle-v1-json";
pub const MAX_UNIT_OBJ_ALTERNATE_BATTLE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_ID: &str = "graphics.unit-obj-script-special-normal";
pub const UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FILE: &str = "unit-obj-script-special-normal.json";
pub const UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FORMAT: &str = "unit-obj-script-special-normal-v1-json";
pub const MAX_UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_ID: &str = "graphics.unit-obj-script-special-alternate";
pub const UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FILE: &str = "unit-obj-script-special-alternate.json";
pub const UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FORMAT: &str = "unit-obj-script-special-alternate-v1-json";
pub const MAX_UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_SCRIPT_OVERRIDE_ID: &str = "graphics.unit-obj-script-override";
pub const UNIT_OBJ_SCRIPT_OVERRIDE_FILE: &str = "unit-obj-script-override.json";
pub const UNIT_OBJ_SCRIPT_OVERRIDE_FORMAT: &str = "unit-obj-script-override-v1-json";
pub const MAX_UNIT_OBJ_SCRIPT_OVERRIDE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_ID: &str = "graphics.unit-obj-script-registry-special";
pub const UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FILE: &str = "unit-obj-script-registry-special.json";
pub const UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FORMAT: &str = "unit-obj-script-registry-special-v1-json";
pub const MAX_UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_ID: &str = "graphics.unit-obj-script-registry-generic";
pub const UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FILE: &str = "unit-obj-script-registry-generic.json";
pub const UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FORMAT: &str = "unit-obj-script-registry-generic-v1-json";
pub const MAX_UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_PROPERTY_WORDS_ID: &str = "graphics.unit-obj-property-words";
pub const UNIT_OBJ_PROPERTY_WORDS_FILE: &str = "unit-obj-property-words.json";
pub const UNIT_OBJ_PROPERTY_WORDS_FORMAT: &str = "unit-obj-property-words-v1-json";
pub const MAX_UNIT_OBJ_PROPERTY_WORDS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_BYTE_PROPERTY_WORDS_ID: &str = "graphics.unit-obj-byte-property-words";
pub const UNIT_OBJ_BYTE_PROPERTY_WORDS_FILE: &str = "unit-obj-byte-property-words.json";
pub const UNIT_OBJ_BYTE_PROPERTY_WORDS_FORMAT: &str = "unit-obj-byte-property-words-v1-json";
pub const MAX_UNIT_OBJ_BYTE_PROPERTY_WORDS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UNIT_OBJ_FFFF_READER_FIELDS_ID: &str = "graphics.unit-obj-ffff-reader-fields";
pub const UNIT_OBJ_FFFF_READER_FIELDS_FILE: &str = "unit-obj-ffff-reader-fields.json";
pub const UNIT_OBJ_FFFF_READER_FIELDS_FORMAT: &str = "unit-obj-ffff-reader-fields-v1-json";
pub const MAX_UNIT_OBJ_FFFF_READER_FIELDS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const SLOT_JOB_LOOKUP_ID: &str = "gameplay.slot-job-lookup";
pub const SLOT_JOB_LOOKUP_FILE: &str = "slot-job-lookup.json";
pub const SLOT_JOB_LOOKUP_FORMAT: &str = "slot-job-lookup-v1-json";
pub const MAX_SLOT_JOB_LOOKUP_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const OBSERVED_TASK_PARAMETERS_ID: &str = "gameplay.observed-task-parameters";
pub const OBSERVED_TASK_PARAMETERS_FILE: &str = "observed-task-parameters.json";
pub const OBSERVED_TASK_PARAMETERS_FORMAT: &str = "observed-task-parameters-v1-json";
pub const MAX_OBSERVED_TASK_PARAMETERS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const CONSTRUCTOR_OBJ_ANIMATION_ID: &str = "graphics.constructor-obj-animation";
pub const CONSTRUCTOR_OBJ_ANIMATION_FILE: &str = "constructor-obj-animation.json";
pub const CONSTRUCTOR_OBJ_ANIMATION_FORMAT: &str = "constructor-obj-animation-v1-json";
pub const MAX_CONSTRUCTOR_OBJ_ANIMATION_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const TASK_CYCLE_DELTAS_ID: &str = "gameplay.task-cycle-deltas";
pub const TASK_CYCLE_DELTAS_FILE: &str = "task-cycle-deltas.json";
pub const TASK_CYCLE_DELTAS_FORMAT: &str = "task-cycle-deltas-v1-json";
pub const MAX_TASK_CYCLE_DELTAS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const UI_RAW_TRANSFERS_ID: &str = "graphics.ui-raw-transfers";
pub const UI_RAW_TRANSFERS_FILE: &str = "ui-raw-transfers.json";
pub const UI_RAW_TRANSFERS_FORMAT: &str = "ui-raw-transfers-v1-json";
pub const MAX_UI_RAW_TRANSFERS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const DYNAMIC_RAW_FRAGMENTS_ID: &str = "graphics.dynamic-raw-fragments";
pub const DYNAMIC_RAW_FRAGMENTS_FILE: &str = "dynamic-raw-fragments.json";
pub const DYNAMIC_RAW_FRAGMENTS_FORMAT: &str = "dynamic-raw-fragments-v1-json";
pub const MAX_DYNAMIC_RAW_FRAGMENTS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const CONDITIONAL_FOUR_BYTE_LOOKUP_ID: &str = "gameplay.conditional-four-byte-lookup";
pub const CONDITIONAL_FOUR_BYTE_LOOKUP_FILE: &str = "conditional-four-byte-lookup.json";
pub const CONDITIONAL_FOUR_BYTE_LOOKUP_FORMAT: &str = "conditional-four-byte-lookup-v1-json";
pub const MAX_CONDITIONAL_FOUR_BYTE_LOOKUP_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const CONDITIONAL_PACKED_LOOKUP_ID: &str = "gameplay.conditional-packed-lookup";
pub const CONDITIONAL_PACKED_LOOKUP_FILE: &str = "conditional-packed-lookup.json";
pub const CONDITIONAL_PACKED_LOOKUP_FORMAT: &str = "conditional-packed-lookup-v1-json";
pub const MAX_CONDITIONAL_PACKED_LOOKUP_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const CONDITIONAL_DESCRIPTOR_BANKS_ID: &str = "gameplay.conditional-descriptor-banks";
pub const CONDITIONAL_DESCRIPTOR_BANKS_FILE: &str = "conditional-descriptor-banks.json";
pub const CONDITIONAL_DESCRIPTOR_BANKS_FORMAT: &str = "conditional-descriptor-banks-v1-json";
pub const MAX_CONDITIONAL_DESCRIPTOR_BANKS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const CONDITIONAL_DESCRIPTOR_BANK_ZERO_ID: &str = "gameplay.conditional-descriptor-bank-zero";
pub const CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE: &str = "conditional-descriptor-bank-zero.json";
pub const CONDITIONAL_DESCRIPTOR_BANK_ZERO_FORMAT: &str = "conditional-descriptor-bank-zero-v1-json";
pub const MAX_CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const CONDITIONAL_PROPERTY_MASKS_ID: &str = "gameplay.conditional-property-masks";
pub const CONDITIONAL_PROPERTY_MASKS_FILE: &str = "conditional-property-masks.json";
pub const CONDITIONAL_PROPERTY_MASKS_FORMAT: &str = "conditional-property-masks-v1-json";
pub const MAX_CONDITIONAL_PROPERTY_MASKS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const COMPACT_OBJ_ANIMATIONS_ID: &str = "graphics.conditional-compact-obj-animations";
pub const COMPACT_OBJ_ANIMATIONS_FILE: &str = "compact-obj-animations.json";
pub const COMPACT_OBJ_ANIMATIONS_FORMAT: &str = "compact-obj-animations-v1-json";
pub const MAX_COMPACT_OBJ_ANIMATIONS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const SECONDARY_COMPACT_OBJ_ANIMATIONS_ID: &str = "graphics.conditional-compact-obj-animations-secondary";
pub const SECONDARY_COMPACT_OBJ_ANIMATIONS_FILE: &str = "compact-obj-animations-secondary.json";
pub const SECONDARY_COMPACT_OBJ_ANIMATIONS_FORMAT: &str = "compact-obj-animations-secondary-v1-json";
pub const MAX_SECONDARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const TERTIARY_COMPACT_OBJ_ANIMATIONS_ID: &str = "graphics.conditional-compact-obj-animations-tertiary";
pub const TERTIARY_COMPACT_OBJ_ANIMATIONS_FILE: &str = "compact-obj-animations-tertiary.json";
pub const TERTIARY_COMPACT_OBJ_ANIMATIONS_FORMAT: &str = "compact-obj-animations-tertiary-v1-json";
pub const MAX_TERTIARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const QUATERNARY_COMPACT_OBJ_ANIMATIONS_ID: &str = "graphics.conditional-compact-obj-animations-quaternary";
pub const QUATERNARY_COMPACT_OBJ_ANIMATIONS_FILE: &str = "compact-obj-animations-quaternary.json";
pub const QUATERNARY_COMPACT_OBJ_ANIMATIONS_FORMAT: &str = "compact-obj-animations-quaternary-v1-json";
pub const MAX_QUATERNARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const QUINARY_COMPACT_OBJ_ANIMATIONS_ID: &str = "graphics.conditional-compact-obj-animations-quinary";
pub const QUINARY_COMPACT_OBJ_ANIMATIONS_FILE: &str = "compact-obj-animations-quinary.json";
pub const QUINARY_COMPACT_OBJ_ANIMATIONS_FORMAT: &str = "compact-obj-animations-quinary-v1-json";
pub const MAX_QUINARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const SENARY_COMPACT_OBJ_ANIMATIONS_ID: &str = "graphics.conditional-compact-obj-animations-senary";
pub const SENARY_COMPACT_OBJ_ANIMATIONS_FILE: &str = "compact-obj-animations-senary.json";
pub const SENARY_COMPACT_OBJ_ANIMATIONS_FORMAT: &str = "compact-obj-animations-senary-v1-json";
pub const MAX_SENARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const INDEXED_GRAPHICS_ID: &str = "graphics.indexed-archives";
pub const INDEXED_GRAPHICS_FILE: &str = "indexed-graphics.json";
pub const INDEXED_GRAPHICS_FORMAT: &str = "indexed-graphics-v1-json";
pub const MAX_INDEXED_GRAPHICS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const WORLD_RECORDS38_ID: &str = "gameplay.world-records38";
pub const WORLD_RECORDS38_FILE: &str = "world-records38.json";
pub const WORLD_RECORDS38_FORMAT: &str = "world-records38-v1-json";
pub const MAX_WORLD_RECORDS38_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const PARAMETER_RECORDS27_ID: &str = "gameplay.parameter-records27";
pub const PARAMETER_RECORDS27_FILE: &str = "parameter-records27.json";
pub const PARAMETER_RECORDS27_FORMAT: &str = "parameter-records27-v1-json";
pub const MAX_PARAMETER_RECORDS27_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const EVENT_RECORDS12_ID: &str = "gameplay.event-records12";
pub const EVENT_RECORDS12_FILE: &str = "event-records12.json";
pub const EVENT_RECORDS12_FORMAT: &str = "event-records12-v1-json";
pub const MAX_EVENT_RECORDS12_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const SELECTION_BYTE_TABLES_ID: &str = "gameplay.selection-byte-tables";
pub const SELECTION_BYTE_TABLES_FILE: &str = "selection-byte-tables.json";
pub const SELECTION_BYTE_TABLES_FORMAT: &str = "selection-byte-tables-v1-json";
pub const MAX_SELECTION_BYTE_TABLES_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MISSION_RECORDS70_ID: &str = "gameplay.mission-records70";
pub const MISSION_RECORDS70_FILE: &str = "mission-records70.json";
pub const MISSION_RECORDS70_FORMAT: &str = "mission-records70-v1-json";
pub const MAX_MISSION_RECORDS70_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const CONDITIONAL_RECORDS8_ID: &str = "gameplay.conditional-records8";
pub const CONDITIONAL_RECORDS8_FILE: &str = "conditional-records8.json";
pub const CONDITIONAL_RECORDS8_FORMAT: &str = "conditional-records8-v1-json";
pub const MAX_CONDITIONAL_RECORDS8_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const RECORDS4_ID: &str = "gameplay.linked-records4";
pub const RECORDS4_FILE: &str = "linked-records4.json";
pub const RECORDS4_FORMAT: &str = "linked-records4-v1-json";
pub const MAX_RECORDS4_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const METADATA28_ID: &str = "gameplay.linked-metadata28";
pub const METADATA28_FILE: &str = "linked-metadata28.json";
pub const METADATA28_FORMAT: &str = "linked-metadata28-v1-json";
pub const MAX_METADATA28_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const ENCOUNTER_COORDINATES_ID: &str = "gameplay.encounter-coordinates";
pub const ENCOUNTER_COORDINATES_FILE: &str = "encounter-coordinates.json";
pub const ENCOUNTER_COORDINATES_FORMAT: &str = "encounter-coordinates-v1-json";
pub const MAX_ENCOUNTER_COORDINATES_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const ENCOUNTER_UNIT_TEMPLATES_ID: &str = "gameplay.encounter-unit-templates";
pub const ENCOUNTER_UNIT_TEMPLATES_FILE: &str = "encounter-unit-templates.json";
pub const ENCOUNTER_UNIT_TEMPLATES_FORMAT: &str = "encounter-unit-templates-v1-json";
pub const MAX_ENCOUNTER_UNIT_TEMPLATES_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const ENCOUNTER_HEADERS_ID: &str = "gameplay.encounter-headers";
pub const ENCOUNTER_HEADERS_FILE: &str = "encounter-headers.json";
pub const ENCOUNTER_HEADERS_FORMAT: &str = "encounter-headers-v1-json";
pub const MAX_ENCOUNTER_HEADERS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const WORLD_ROADS_ID: &str = "gameplay.world-roads";
pub const WORLD_ROADS_FILE: &str = "world-roads.json";
pub const WORLD_ROADS_FORMAT: &str = "world-roads-v1-json";
pub const MAX_WORLD_ROADS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const WORLD_NODES_ID: &str = "gameplay.world-nodes";
pub const WORLD_NODES_FILE: &str = "world-nodes.json";
pub const WORLD_NODES_FORMAT: &str = "world-nodes-v1-json";
pub const MAX_WORLD_NODES_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const WORD_TASK_OBJ_ANIMATIONS_ID: &str = "graphics.word-task-obj-animations";
pub const WORD_TASK_OBJ_ANIMATIONS_FILE: &str = "word-task-obj-animations.json";
pub const WORD_TASK_OBJ_ANIMATIONS_FORMAT: &str = "word-task-obj-animations-v1-json";
pub const MAX_WORD_TASK_OBJ_ANIMATIONS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAP_RASTER_PALETTES_ID: &str = "graphics.map-raster-palettes";
pub const MAP_RASTER_PALETTES_FILE: &str = "map-raster-palettes.json";
pub const MAP_RASTER_PALETTES_FORMAT: &str = "map-raster-palettes-v1-json";
pub const MAX_MAP_RASTER_PALETTES_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const AUXILIARY_COMMANDS_ID: &str = "engine.auxiliary-script-command-directory";
pub const AUXILIARY_COMMANDS_FILE: &str = "auxiliary-script-command-directory.json";
pub const AUXILIARY_COMMANDS_FORMAT: &str = "auxiliary-script-command-directory-v1-json";
pub const MAX_AUXILIARY_COMMANDS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const AUXILIARY_ARCHIVE_ID: &str = "engine.auxiliary-script-archive";
pub const AUXILIARY_ARCHIVE_FILE: &str = "auxiliary-script-archive.json";
pub const AUXILIARY_ARCHIVE_FORMAT: &str = "auxiliary-script-archive-v1-json";
pub const MAX_AUXILIARY_ARCHIVE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const ITEM_LINKS_ID: &str = "gameplay.item-links";
pub const ITEM_LINKS_FILE: &str = "item-links.json";
pub const ITEM_LINKS_FORMAT: &str = "item-links-v1-json";
pub const MAX_ITEM_LINKS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const ITEM_RECORDS_ID: &str = "gameplay.item-records";
pub const ITEM_RECORDS_FILE: &str = "items.json";
pub const ITEM_RECORDS_FORMAT: &str = "item-records-v1-json";
pub const MAX_ITEM_RECORDS_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const UI_CUSTOM_GRAPHICS_ID: &str = "graphics.ui-custom-banks";
pub const UI_CUSTOM_GRAPHICS_FILE: &str = "ui-graphics.json";
pub const UI_CUSTOM_GRAPHICS_FORMAT: &str = "ui-custom-banks-v1-json";
pub const MAX_UI_CUSTOM_GRAPHICS_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub const UI_RAW_PALETTES_ID: &str = "graphics.ui-raw-palettes";
pub const UI_RAW_PALETTES_FILE: &str = "ui-palettes.json";
pub const UI_RAW_PALETTES_FORMAT: &str = "ui-raw-palettes-v1-json";
pub const MAX_UI_RAW_PALETTES_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub const OBJ_CUSTOM_GRAPHICS_ID: &str = "graphics.obj-custom-banks";
pub const OBJ_CUSTOM_GRAPHICS_FILE: &str = "obj-graphics.json";
pub const OBJ_CUSTOM_GRAPHICS_FORMAT: &str = "obj-custom-banks-v1-json";
pub const MAX_OBJ_CUSTOM_GRAPHICS_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub const OBJ_ANIMATION_BANKS_ID: &str = "graphics.obj-animation-banks";
pub const OBJ_ANIMATION_BANKS_FILE: &str = "obj-animation-banks.json";
pub const OBJ_ANIMATION_BANKS_FORMAT: &str = "obj-animation-banks-v1-json";
pub const MAX_OBJ_ANIMATION_BANKS_FILE_BYTES: u64 = 4 * 1024 * 1024;
pub const OBJ_STATIC_LISTS_ID: &str = "graphics.obj-static-lists";
pub const OBJ_STATIC_LISTS_FILE: &str = "obj-static-lists.json";
pub const OBJ_STATIC_LISTS_FORMAT: &str = "obj-static-lists-v1-json";
pub const MAX_OBJ_STATIC_LISTS_FILE_BYTES: u64 = 4 * 1024 * 1024;
pub const OBJ_DIMENSIONS_ID: &str = "graphics.obj-dimensions";
pub const OBJ_DIMENSIONS_FILE: &str = "obj-dimensions.toml";
pub const BG_DESCRIPTORS_ID: &str = "graphics.bg-descriptors";
pub const BG_DESCRIPTORS_FILE: &str = "bg-descriptors.json";
pub const BG_DESCRIPTORS_FORMAT: &str = "bg-descriptors-json-v2";
pub const BG_BASE_PALETTES_ID: &str = "graphics.bg-base-palettes";
pub const BG_BASE_PALETTES_FILE: &str = "bg-base-palettes.json";
pub const BG_BASE_PALETTES_FORMAT: &str = "bg-base-palettes-v2-json";
pub const BG_ALTERNATE_PALETTES_ID: &str = "graphics.bg-alternate-palettes";
pub const BG_ALTERNATE_PALETTES_FILE: &str = "bg-alternate-palettes.json";
pub const BG_ALTERNATE_PALETTES_FORMAT: &str = "bg-alternate-palettes-v1-json";
pub const BG_PACKET_PROGRAMS_ID: &str = "graphics.bg-packet-programs";
pub const BG_PACKET_PROGRAMS_FILE: &str = "bg-packet-programs.json";
pub const BG_PACKET_PROGRAMS_FORMAT: &str = "bg-packet-programs-v1-json";
pub const BG_MAIN_GRAPHICS_ID: &str = "graphics.bg-main-graphics";
pub const BG_MAIN_GRAPHICS_FILE: &str = "bg-main-graphics.json";
pub const BG_MAIN_GRAPHICS_FORMAT: &str = "bg-main-graphics-v1-json";
pub const MAX_BG_MAIN_GRAPHICS_FILE_BYTES: u64 = 32 * 1024 * 1024;
pub const FONT_BANK_ID: &str = "font.complete-bank";
pub const FONT_BANK_FILE: &str = "font-bank.json";
pub const FONT_BANK_FORMAT: &str = "font-bank-v1-json";
pub const MAX_FONT_BANK_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub const NAME_KEYBOARD_ID: &str = "text.name-keyboard";
pub const NAME_KEYBOARD_FILE: &str = "name-keyboard.json";
pub const NAME_KEYBOARD_FORMAT: &str = "name-keyboard-v1-json";
pub const MAX_NAME_KEYBOARD_FILE_BYTES: u64 = 1 * 1024 * 1024;
pub const TEXT_CATALOGS_ID: &str = "text.catalogs";
pub const TEXT_CATALOGS_FILE: &str = "text-catalogs.json";
pub const TEXT_CATALOGS_FORMAT: &str = "text-catalogs-v1-json";
pub const MAX_TEXT_CATALOGS_FILE_BYTES: u64 = 512 * 1024 * 1024;
pub const SHORT_TEXTS_ID: &str = "text.short-strings";
pub const SHORT_TEXTS_FILE: &str = "short-texts.json";
pub const SHORT_TEXTS_FORMAT: &str = "short-texts-v1-json";
pub const MAX_SHORT_TEXTS_FILE_BYTES: u64 = 8 * 1024 * 1024;
pub const BG_TRANSFER_PROGRAMS_ID: &str = "graphics.bg-transfer-programs";
pub const BG_TRANSFER_PROGRAMS_FILE: &str = "bg-transfer-programs.json";
pub const BG_TRANSFER_PROGRAMS_FORMAT: &str = "bg-transfer-programs-v1-json";
pub const MAX_BG_TRANSFER_PROGRAMS_FILE_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_BG_PACKET_PROGRAMS_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_BG_ALTERNATE_PALETTES_FILE_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_MANIFEST_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_BG_BASE_PALETTES_FILE_BYTES: u64 = 1024 * 1024;
pub const MANIFEST_FILE: &str = "manifest.toml";
pub const MAX_FILE_BYTES: u64 = 16 * 1024;
pub const MAX_BG_DESCRIPTORS_FILE_BYTES: u64 = 256 * 1024;
pub const MAX_DATASET_BYTES: usize = 512 * 1024 * 1024;
pub const DIMENSIONS_SHA256: &str =
    "6ff886b23cde2726a4ceb9cba8c5569bbb4dc6a1185769547f78dfbcb7e2d991";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjSize {
    pub width: u16,
    pub height: u16,
}

/// Shape-major, size-minor indexed metadata. Shape 3 has no record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjDimensions {
    pub entries: [ObjSize; 12],
}

impl ObjDimensions {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 48 {
            return Err("OBJ dimensions require exactly 48 source bytes".into());
        }
        let result = Self {
            entries: std::array::from_fn(|i| ObjSize {
                width: u16::from_le_bytes([bytes[i * 4], bytes[i * 4 + 1]]),
                height: u16::from_le_bytes([bytes[i * 4 + 2], bytes[i * 4 + 3]]),
            }),
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), String> {
        // Hardware OBJ extents; the actual base ordering is authenticated by
        // the source digest, independently of these structural constraints.
        if self
            .entries
            .iter()
            .any(|e| !matches!(e.width, 8 | 16 | 32 | 64) || !matches!(e.height, 8 | 16 | 32 | 64))
        {
            return Err("Invalid OBJ dimension (expected 8, 16, 32 or 64 pixels)".into());
        }
        Ok(())
    }

    pub fn source_bytes(&self) -> [u8; 48] {
        let mut out = [0; 48];
        for (i, e) in self.entries.iter().enumerate() {
            out[i * 4..i * 4 + 2].copy_from_slice(&e.width.to_le_bytes());
            out[i * 4 + 2..i * 4 + 4].copy_from_slice(&e.height.to_le_bytes());
        }
        out
    }

    pub fn get(&self, shape: u8, size: u8) -> Option<&ObjSize> {
        if shape >= 3 || size >= 4 {
            return None;
        }
        self.entries.get(shape as usize * 4 + size as usize)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRange {
    pub offset: u32,
    pub length: u32,
    pub sha256: String,
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub id: String,
    pub path: String,
    pub format: String,
    pub sha256: String,
    pub dependencies: Vec<String>,
    pub sources: Vec<SourceRange>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub extractor_revision: u32,
    pub decoder_revision: u32,
    pub source_rom_sha256: String,
    pub source_rom_size: u32,
    pub profile: String,
    pub coverage: Vec<String>,
    pub resources: Vec<Resource>,
}

/// A source-authenticated extraction. This is not a mod loader.
pub struct Extraction {
    pub manifest: Manifest,
    pub dimensions: ObjDimensions,
    pub background: Option<bg_descriptors::Descriptors>,
    pub palettes: Option<bg_palettes::Catalog>,
    pub alternate_palettes: Option<bg_alternate_palettes::Catalog>,
    pub packet_programs: Option<bg_packet_data::Catalog>,
    pub main_graphics: Option<bg_graphics::Catalog>,
    pub transfers: Option<bg_transfers::Catalog>,
    pub short_texts: Option<text_short::Catalog>,
    pub font_bank: Option<fonts::Catalog>,
    pub name_keyboard: Option<name_keyboard::Catalog>,
    pub obj_lists: Option<obj_lists::Catalog>,
    pub obj_animations: Option<obj_animations::Catalog>,
    pub obj_graphics: Option<obj_graphics::Catalog>,
    pub ui_palettes: Option<ui_palettes::Catalog>,
    pub ui_graphics: Option<ui_graphics::Catalog>,
    pub items: Option<items::Catalog>,
    pub item_links: Option<item_links::Catalog>,
    pub metadata16: Option<metadata16::Catalog>,
    pub metadata52: Option<metadata52::Catalog>,
    pub metadata4: Option<metadata4::Catalog>,
    pub metadata2: Option<metadata2::Catalog>,
    pub default48: Option<default48::Catalog>,
    pub constructor_alternates: Option<constructor_alternates::Catalog>,
    pub metadata14: Option<metadata14::Catalog>,
    pub metadata12: Option<metadata12::Catalog>,
    pub global_pair_flags: Option<global_pair_flags::Catalog>,
    pub evaluation_parameters: Option<evaluation_parameters::Catalog>,
    pub progression_rewards: Option<progression_rewards::Catalog>,
    pub progression_gates: Option<progression_gates::Catalog>,
    pub progression_selection_gates: Option<progression_selection_gates::Catalog>,
    pub aggregate_flag_keys: Option<aggregate_flag_keys::Catalog>,
    pub trade_parameters: Option<trade_parameters::Catalog>,
    pub item_effect_parameters: Option<item_effect_parameters::Catalog>,
    pub audio_directory: Option<audio_directory::Catalog>,
    pub audio_tones: Option<audio_tones::Catalog>,
    pub audio_sequences: Option<audio_sequences::Catalog>,
    pub audio_driver: Option<audio_driver::Catalog>,
    pub palette_effect_dispatch: Option<palette_effect_dispatch::Catalog>,
    pub palette_programs: Option<palette_programs::Catalog>,
    pub blend_programs: Option<blend_programs::Catalog>,
    pub task_descriptors: Option<task_descriptors::Catalog>,
    pub effect_event_directories: Option<effect_event_directories::Catalog>,
    pub script_commands: Option<script_commands::Catalog>,
    pub script_archive: Option<script_archive::Catalog>,
    pub script_text: Option<script_text::Catalog>,
    pub script_coordinates: Option<script_coordinates::Catalog>,
    pub ui_choice_lists: Option<ui_choice_lists::Catalog>,
    pub unit_obj_initial_pose: Option<unit_obj_initial_pose::Catalog>,
    pub color_mode_palettes: Option<color_mode_palettes::Catalog>,
    pub unit_obj_equipment: Option<unit_obj_equipment::Catalog>,
    pub unit_obj_normal_battle: Option<unit_obj_normal_battle::Catalog>,
    pub unit_obj_alternate_battle: Option<unit_obj_alternate_battle::Catalog>,
    pub unit_obj_script_special_normal: Option<unit_obj_script_special_normal::Catalog>,
    pub unit_obj_script_special_alternate: Option<unit_obj_script_special_alternate::Catalog>,
    pub unit_obj_script_override: Option<unit_obj_script_override::Catalog>,
    pub unit_obj_script_registry_special: Option<unit_obj_script_registry_special::Catalog>,
    pub unit_obj_script_registry_generic: Option<unit_obj_script_registry_generic::Catalog>,
    pub unit_obj_property_words: Option<unit_obj_property_words::Catalog>,
    pub unit_obj_byte_property_words: Option<unit_obj_byte_property_words::Catalog>,
    pub unit_obj_ffff_reader_fields: Option<unit_obj_ffff_reader_fields::Catalog>,
    pub slot_job_lookup: Option<slot_job_lookup::Catalog>,
    pub observed_task_parameters: Option<observed_task_parameters::Catalog>,
    pub constructor_obj_animation: Option<obj_lists::constructor_animation::Catalog>,
    pub task_cycle_deltas: Option<task_cycle_deltas::Catalog>,
    pub word_task_obj_animations: Option<word_task_obj_animations::Catalog>,
    pub world_nodes: Option<world_nodes::Catalog>,
    pub world_roads: Option<world_roads::Catalog>,
    pub encounter_headers: Option<encounter_headers::Catalog>,
    pub encounter_unit_templates: Option<encounter_unit_templates::Catalog>,
    pub encounter_coordinates: Option<encounter_coordinates::Catalog>,
    pub metadata28: Option<metadata28::Catalog>,
    pub records4: Option<records4::Catalog>,
    pub conditional_records8: Option<conditional_records8::Catalog>,
    pub mission_records70: Option<mission_records70::Catalog>,
    pub selection_byte_tables: Option<selection_byte_tables::Catalog>,
    pub event_records12: Option<event_records12::Catalog>,
    pub parameter_records27: Option<parameter_records27::Catalog>,
    pub world_records38: Option<world_records38::Catalog>,
    pub indexed_graphics: Option<indexed_graphics::Catalog>,
    pub ui_raw_transfers: Option<ui_raw_transfers::Catalog>,
    pub dynamic_raw_fragments: Option<ui_raw_transfers::dynamic_fragments::Catalog>,
    pub conditional_four_byte_lookup: Option<conditional_four_byte_lookup::Catalog>,
    pub conditional_packed_lookup: Option<conditional_packed_lookup::Catalog>,
    pub conditional_descriptor_banks: Option<conditional_descriptor_banks::Catalog>,
    pub conditional_descriptor_bank_zero: Option<conditional_descriptor_bank_zero::Catalog>,
    pub conditional_property_masks: Option<conditional_property_masks::Catalog>,
    pub compact_obj_animations: Option<compact_obj_animations::Catalog>,
    pub secondary_compact_obj_animations: Option<compact_obj_animations_secondary::Catalog>,
    pub tertiary_compact_obj_animations: Option<compact_obj_animations_tertiary::Catalog>,
    pub quaternary_compact_obj_animations: Option<compact_obj_animations_quaternary::Catalog>,
    pub quinary_compact_obj_animations: Option<compact_obj_animations_quinary::Catalog>,
    pub senary_compact_obj_animations: Option<compact_obj_animations_senary::Catalog>,
    pub map_raster_palettes: Option<map_raster_palettes::Catalog>,
    pub auxiliary_scripts: Option<auxiliary_scripts::Catalog>,
    pub text_catalogs: Option<text_catalogs::Catalog>,
}

/// Typed values borrowed from the exact captured file bytes. The frontend
/// parses them once; source validation and runtime binding cover the whole set.
#[derive(Clone, Copy)]
pub struct BaseFiles<'a> {
    pub manifest: &'a Manifest,
    pub dimensions: (&'a ObjDimensions, &'a [u8]),
    pub background: Option<(&'a bg_descriptors::Descriptors, &'a [u8])>,
    pub palettes: Option<(&'a bg_palettes::Palettes, &'a [u8])>,
    pub alternate_palettes: Option<(&'a bg_alternate_palettes::Palettes, &'a [u8])>,
    pub packet_programs: Option<(&'a bg_packet_data::Programs, &'a [u8])>,
    pub main_graphics: Option<(&'a bg_graphics::Graphics, &'a [u8])>,
    pub transfers: Option<(&'a bg_transfers::Transfers, &'a [u8])>,
    pub short_texts: Option<(&'a text_short::ShortTexts, &'a [u8])>,
    pub font_bank: Option<(&'a fonts::FontBank, &'a [u8])>,
    pub name_keyboard: Option<(&'a name_keyboard::Keyboard, &'a [u8])>,
    pub obj_lists: Option<(&'a obj_lists::Lists, &'a [u8])>,
    pub obj_animations: Option<(&'a obj_animations::Banks, &'a [u8])>,
    pub obj_graphics: Option<(&'a obj_graphics::Graphics, &'a [u8])>,
    pub ui_palettes: Option<(&'a ui_palettes::Palettes, &'a [u8])>,
    pub ui_graphics: Option<(&'a ui_graphics::Graphics, &'a [u8])>,
    pub items: Option<(&'a items::Items, &'a [u8])>,
    pub item_links: Option<(&'a item_links::Links, &'a [u8])>,
    pub metadata16: Option<(&'a metadata16::Metadata16, &'a [u8])>,
    pub metadata52: Option<(&'a metadata52::Metadata52, &'a [u8])>,
    pub metadata4: Option<(&'a metadata4::Metadata4, &'a [u8])>,
    pub metadata2: Option<(&'a metadata2::Metadata2, &'a [u8])>,
    pub default48: Option<(&'a default48::Template, &'a [u8])>,
    pub constructor_alternates: Option<(&'a constructor_alternates::Alternates, &'a [u8])>,
    pub metadata14: Option<(&'a metadata14::Metadata14, &'a [u8])>,
    pub metadata12: Option<(&'a metadata12::Metadata12, &'a [u8])>,
    pub global_pair_flags: Option<(&'a global_pair_flags::GlobalPairFlags, &'a [u8])>,
    pub evaluation_parameters: Option<(&'a evaluation_parameters::EvaluationParameters, &'a [u8])>,
    pub progression_rewards: Option<(&'a progression_rewards::ProgressionRewards, &'a [u8])>,
    pub progression_gates: Option<(&'a progression_gates::ProgressionGates, &'a [u8])>,
    pub progression_selection_gates: Option<(&'a progression_selection_gates::ProgressionSelectionGates, &'a [u8])>,
    pub aggregate_flag_keys: Option<(&'a aggregate_flag_keys::AggregateFlagKeys, &'a [u8])>,
    pub trade_parameters: Option<(&'a trade_parameters::TradeParameters, &'a [u8])>,
    pub item_effect_parameters: Option<(&'a item_effect_parameters::ItemEffectParameters, &'a [u8])>,
    pub audio_directory: Option<(&'a audio_directory::AudioDirectory, &'a [u8])>,
    pub audio_tones: Option<(&'a audio_tones::AudioTones, &'a [u8])>,
    pub audio_sequences: Option<(&'a audio_sequences::AudioSequences, &'a [u8])>,
    pub audio_driver: Option<(&'a audio_driver::AudioDriver, &'a [u8])>,
    pub palette_effect_dispatch: Option<(&'a palette_effect_dispatch::PaletteEffectDispatch, &'a [u8])>,
    pub palette_programs: Option<(&'a palette_programs::PalettePrograms, &'a [u8])>,
    pub blend_programs: Option<(&'a blend_programs::BlendPrograms, &'a [u8])>,
    pub task_descriptors: Option<(&'a task_descriptors::TaskDescriptors, &'a [u8])>,
    pub effect_event_directories: Option<(&'a effect_event_directories::EffectEventDirectories, &'a [u8])>,
    pub script_commands: Option<(&'a script_commands::ScriptCommands, &'a [u8])>,
    pub script_archive: Option<(&'a script_archive::ScriptArchive, &'a [u8])>,
    pub script_text: Option<(&'a script_text::ScriptText, &'a [u8])>,
    pub script_coordinates: Option<(&'a script_coordinates::ScriptCoordinates, &'a [u8])>,
    pub ui_choice_lists: Option<(&'a ui_choice_lists::UiChoiceLists, &'a [u8])>,
    pub unit_obj_initial_pose: Option<(&'a unit_obj_initial_pose::UnitObjInitialPose, &'a [u8])>,
    pub color_mode_palettes: Option<(&'a color_mode_palettes::ColorModePalettes, &'a [u8])>,
    pub unit_obj_equipment: Option<(&'a unit_obj_equipment::UnitObjEquipment, &'a [u8])>,
    pub unit_obj_normal_battle: Option<(&'a unit_obj_normal_battle::UnitObjNormalBattle, &'a [u8])>,
    pub unit_obj_alternate_battle: Option<(&'a unit_obj_alternate_battle::UnitObjAlternateBattle, &'a [u8])>,
    pub unit_obj_script_special_normal: Option<(&'a unit_obj_script_special_normal::UnitObjScriptSpecialNormal, &'a [u8])>,
    pub unit_obj_script_special_alternate: Option<(&'a unit_obj_script_special_alternate::UnitObjScriptSpecialAlternate, &'a [u8])>,
    pub unit_obj_script_override: Option<(&'a unit_obj_script_override::UnitObjScriptOverride, &'a [u8])>,
    pub unit_obj_script_registry_special: Option<(&'a unit_obj_script_registry_special::UnitObjScriptRegistrySpecial, &'a [u8])>,
    pub unit_obj_script_registry_generic: Option<(&'a unit_obj_script_registry_generic::UnitObjScriptRegistryGeneric, &'a [u8])>,
    pub unit_obj_property_words: Option<(&'a unit_obj_property_words::UnitObjPropertyWords, &'a [u8])>,
    pub unit_obj_byte_property_words: Option<(&'a unit_obj_byte_property_words::UnitObjBytePropertyWords, &'a [u8])>,
    pub unit_obj_ffff_reader_fields: Option<(&'a unit_obj_ffff_reader_fields::UnitObjFfffReaderFields, &'a [u8])>,
    pub slot_job_lookup: Option<(&'a slot_job_lookup::SlotJobLookup, &'a [u8])>,
    pub observed_task_parameters: Option<(&'a observed_task_parameters::ObservedTaskParameters, &'a [u8])>,
    pub constructor_obj_animation: Option<(&'a obj_lists::constructor_animation::Animation, &'a [u8])>,
    pub task_cycle_deltas: Option<(&'a task_cycle_deltas::TaskCycleDeltas, &'a [u8])>,
    pub word_task_obj_animations: Option<(&'a word_task_obj_animations::WordTaskObjAnimations, &'a [u8])>,
    pub world_nodes: Option<(&'a world_nodes::WorldNodes, &'a [u8])>,
    pub world_roads: Option<(&'a world_roads::WorldRoads, &'a [u8])>,
    pub encounter_headers: Option<(&'a encounter_headers::EncounterHeaders, &'a [u8])>,
    pub encounter_unit_templates: Option<(&'a encounter_unit_templates::EncounterUnitTemplates, &'a [u8])>,
    pub encounter_coordinates: Option<(&'a encounter_coordinates::EncounterCoordinates, &'a [u8])>,
    pub metadata28: Option<(&'a metadata28::Metadata28, &'a [u8])>,
    pub records4: Option<(&'a records4::Records4, &'a [u8])>,
    pub conditional_records8: Option<(&'a conditional_records8::Records8, &'a [u8])>,
    pub mission_records70: Option<(&'a mission_records70::Records70, &'a [u8])>,
    pub selection_byte_tables: Option<(&'a selection_byte_tables::ByteTables, &'a [u8])>,
    pub event_records12: Option<(&'a event_records12::EventRecords12, &'a [u8])>,
    pub parameter_records27: Option<(&'a parameter_records27::ParameterRecords27, &'a [u8])>,
    pub world_records38: Option<(&'a world_records38::WorldRecords38, &'a [u8])>,
    pub indexed_graphics: Option<(&'a indexed_graphics::IndexedGraphics, &'a [u8])>,
    pub ui_raw_transfers: Option<(&'a ui_raw_transfers::RawTransfers, &'a [u8])>,
    pub dynamic_raw_fragments: Option<(&'a ui_raw_transfers::dynamic_fragments::Fragments, &'a [u8])>,
    pub conditional_four_byte_lookup: Option<(&'a conditional_four_byte_lookup::Lookup, &'a [u8])>,
    pub conditional_packed_lookup: Option<(&'a conditional_packed_lookup::Lookup, &'a [u8])>,
    pub conditional_descriptor_banks: Option<(&'a conditional_descriptor_banks::DescriptorBanks, &'a [u8])>,
    pub conditional_descriptor_bank_zero: Option<(&'a conditional_descriptor_bank_zero::BankZero, &'a [u8])>,
    pub conditional_property_masks: Option<(&'a conditional_property_masks::Masks, &'a [u8])>,
    pub compact_obj_animations: Option<(&'a compact_obj_animations::CompactAnimations, &'a [u8])>,
    pub secondary_compact_obj_animations: Option<(&'a compact_obj_animations_secondary::CompactAnimationsSecondary, &'a [u8])>,
    pub tertiary_compact_obj_animations: Option<(&'a compact_obj_animations_tertiary::CompactAnimationsTertiary, &'a [u8])>,
    pub quaternary_compact_obj_animations: Option<(&'a compact_obj_animations_quaternary::CompactAnimationsQuaternary, &'a [u8])>,
    pub quinary_compact_obj_animations: Option<(&'a compact_obj_animations_quinary::CompactAnimationsQuinary, &'a [u8])>,
    pub senary_compact_obj_animations: Option<(&'a compact_obj_animations_senary::CompactAnimationsSenary, &'a [u8])>,
    pub map_raster_palettes: Option<(&'a map_raster_palettes::MapRasterPalettes, &'a [u8])>,
    pub auxiliary_commands: Option<(&'a auxiliary_scripts::AuxiliaryCommands, &'a [u8])>,
    pub auxiliary_archive: Option<(&'a auxiliary_scripts::AuxiliaryArchive, &'a [u8])>,
    pub text_catalogs: Option<(&'a text_catalogs::TextCatalogs, &'a [u8])>,
}

impl Extraction {
    pub fn from_rom(rom: &[u8]) -> Result<Self, String> {
        let hash = ffta_rom::digest(rom);
        let (profile, ranges): (&str, &[(u32, &str)]) = match hash.as_str() {
            ffta_rom::SHA256 => ("usa", &[(0x149510, "direct"), (0xa398dc, "iwram-copy")]),
            ffta_rom::EUROPE_SHA256 => (
                "europe",
                &[(0x14ced8, "rom-table"), (0xd976e8, "iwram-copy")],
            ),
            _ => return Err("Decoded resources require a verified FFTA EUR or USA ROM".into()),
        };
        let catalog = bg_descriptors::Catalog::from_rom(rom)?;
        let palettes = bg_palettes::Catalog::from_descriptors(rom, &catalog)?;
        let alternates = bg_alternate_palettes::Catalog::from_rom(rom)?;
        let packet_programs = bg_packet_data::Catalog::from_rom(rom)?;
        let mut result = Self::from_ranges(rom, hash, profile, ranges, DIMENSIONS_SHA256)?;
        result.background = Some(catalog.decoded);
        result.manifest.coverage.push(BG_DESCRIPTORS_ID.into());
        result.manifest.resources.push(Resource {
            id: BG_DESCRIPTORS_ID.into(),
            path: BG_DESCRIPTORS_FILE.into(),
            format: BG_DESCRIPTORS_FORMAT.into(),
            sha256: String::new(),
            dependencies: Vec::new(),
            sources: vec![catalog.source],
        });
        result.manifest.coverage.push(BG_BASE_PALETTES_ID.into());
        result.manifest.resources.push(Resource {
            id: BG_BASE_PALETTES_ID.into(),
            path: BG_BASE_PALETTES_FILE.into(),
            format: BG_BASE_PALETTES_FORMAT.into(),
            sha256: String::new(),
            dependencies: vec![BG_DESCRIPTORS_ID.into()],
            sources: palettes
                .sources
                .iter()
                .map(|source| source.range.clone())
                .collect(),
        });
        result.palettes = Some(palettes);
        result.manifest.coverage.push(BG_ALTERNATE_PALETTES_ID.into());
        result.manifest.resources.push(Resource {
            id: BG_ALTERNATE_PALETTES_ID.into(),
            path: BG_ALTERNATE_PALETTES_FILE.into(),
            format: BG_ALTERNATE_PALETTES_FORMAT.into(),
            sha256: String::new(),
            dependencies: vec![BG_DESCRIPTORS_ID.into()],
            sources: alternates.tables.iter().chain(&alternates.sources).cloned().collect(),
        });
        result.alternate_palettes = Some(alternates);
        result.manifest.coverage.push(BG_PACKET_PROGRAMS_ID.into());
        result.manifest.resources.push(Resource {
            id: BG_PACKET_PROGRAMS_ID.into(),
            path: BG_PACKET_PROGRAMS_FILE.into(),
            format: BG_PACKET_PROGRAMS_FORMAT.into(),
            sha256: String::new(),
            dependencies: vec![BG_DESCRIPTORS_ID.into()],
            sources: packet_programs.sources.clone(),
        });
        result.packet_programs = Some(packet_programs);
        // User priority is EUR. Preserve USA's existing dataset contract until
        // its graphics catalog and new managed path are independently verified.
        if profile == "europe" {
            let graphics = bg_graphics::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(BG_MAIN_GRAPHICS_ID.into());
            result.manifest.resources.push(Resource {
                id: BG_MAIN_GRAPHICS_ID.into(), path: BG_MAIN_GRAPHICS_FILE.into(),
                format: BG_MAIN_GRAPHICS_FORMAT.into(), sha256: String::new(),
                dependencies: vec![BG_DESCRIPTORS_ID.into()], sources: graphics.sources.clone(),
            });
            result.main_graphics = Some(graphics);
            let transfers = bg_transfers::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(BG_TRANSFER_PROGRAMS_ID.into());
            result.manifest.resources.push(Resource {
                id: BG_TRANSFER_PROGRAMS_ID.into(), path: BG_TRANSFER_PROGRAMS_FILE.into(),
                format: BG_TRANSFER_PROGRAMS_FORMAT.into(), sha256: String::new(),
                dependencies: vec![BG_DESCRIPTORS_ID.into()], sources: transfers.sources.clone(),
            });
            result.transfers = Some(transfers);
            let short_texts = text_short::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(SHORT_TEXTS_ID.into());
            result.manifest.resources.push(Resource {
                id: SHORT_TEXTS_ID.into(), path: SHORT_TEXTS_FILE.into(),
                format: SHORT_TEXTS_FORMAT.into(), sha256: String::new(),
                dependencies: Vec::new(), sources: short_texts.sources.clone(),
            });
            result.short_texts = Some(short_texts);
            let texts = text_catalogs::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(TEXT_CATALOGS_ID.into());
            result.manifest.resources.push(Resource {
                id: TEXT_CATALOGS_ID.into(), path: TEXT_CATALOGS_FILE.into(), format: TEXT_CATALOGS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![SHORT_TEXTS_ID.into()], sources: texts.sources.clone(),
            });
            result.text_catalogs = Some(texts);
            let font_bank = fonts::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(FONT_BANK_ID.into());
            result.manifest.resources.push(Resource {
                id: FONT_BANK_ID.into(), path: FONT_BANK_FILE.into(), format: FONT_BANK_FORMAT.into(),
                sha256: String::new(), dependencies: vec![TEXT_CATALOGS_ID.into()],
                sources: font_bank.sources.clone(),
            });
            result.font_bank = Some(font_bank);
            let name_keyboard = name_keyboard::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(NAME_KEYBOARD_ID.into());
            result.manifest.resources.push(Resource {
                id: NAME_KEYBOARD_ID.into(), path: NAME_KEYBOARD_FILE.into(), format: NAME_KEYBOARD_FORMAT.into(),
                sha256: String::new(), dependencies: vec![FONT_BANK_ID.into()],
                sources: name_keyboard.sources.clone(),
            });
            result.name_keyboard = Some(name_keyboard);
            let obj_lists = obj_lists::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(OBJ_STATIC_LISTS_ID.into());
            result.manifest.resources.push(Resource {
                id: OBJ_STATIC_LISTS_ID.into(), path: OBJ_STATIC_LISTS_FILE.into(), format: OBJ_STATIC_LISTS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![OBJ_DIMENSIONS_ID.into()], sources: obj_lists.sources.clone(),
            });
            result.obj_lists = Some(obj_lists);
            let animations = obj_animations::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(OBJ_ANIMATION_BANKS_ID.into());
            result.manifest.resources.push(Resource {
                id: OBJ_ANIMATION_BANKS_ID.into(), path: OBJ_ANIMATION_BANKS_FILE.into(), format: OBJ_ANIMATION_BANKS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![OBJ_DIMENSIONS_ID.into()], sources: animations.sources.clone(),
            });
            result.obj_animations = Some(animations);
            let graphics = obj_graphics::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(OBJ_CUSTOM_GRAPHICS_ID.into());
            result.manifest.resources.push(Resource {
                id: OBJ_CUSTOM_GRAPHICS_ID.into(), path: OBJ_CUSTOM_GRAPHICS_FILE.into(), format: OBJ_CUSTOM_GRAPHICS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![OBJ_DIMENSIONS_ID.into()], sources: graphics.sources.clone(),
            });
            result.obj_graphics = Some(graphics);
            let palettes = ui_palettes::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UI_RAW_PALETTES_ID.into());
            result.manifest.resources.push(Resource {
                id: UI_RAW_PALETTES_ID.into(), path: UI_RAW_PALETTES_FILE.into(), format: UI_RAW_PALETTES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![OBJ_DIMENSIONS_ID.into()], sources: palettes.sources.clone(),
            });
            result.ui_palettes = Some(palettes);
            let graphics = ui_graphics::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UI_CUSTOM_GRAPHICS_ID.into());
            result.manifest.resources.push(Resource {
                id: UI_CUSTOM_GRAPHICS_ID.into(), path: UI_CUSTOM_GRAPHICS_FILE.into(), format: UI_CUSTOM_GRAPHICS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![OBJ_DIMENSIONS_ID.into(), OBJ_CUSTOM_GRAPHICS_ID.into()], sources: graphics.sources.clone(),
            });
            result.ui_graphics = Some(graphics);
            let items = items::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(ITEM_RECORDS_ID.into());
            result.manifest.resources.push(Resource {
                id: ITEM_RECORDS_ID.into(), path: ITEM_RECORDS_FILE.into(), format: ITEM_RECORDS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![OBJ_DIMENSIONS_ID.into()], sources: items.sources.clone(),
            });
            result.items = Some(items);
            let links = item_links::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(ITEM_LINKS_ID.into());
            result.manifest.resources.push(Resource {
                id: ITEM_LINKS_ID.into(), path: ITEM_LINKS_FILE.into(), format: ITEM_LINKS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![ITEM_RECORDS_ID.into()], sources: links.sources.clone(),
            });
            result.item_links = Some(links);
            let metadata = metadata16::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(METADATA16_ID.into());
            result.manifest.resources.push(Resource {
                id: METADATA16_ID.into(), path: METADATA16_FILE.into(), format: METADATA16_FORMAT.into(),
                sha256: String::new(), dependencies: vec![OBJ_DIMENSIONS_ID.into()], sources: metadata.sources.clone(),
            });
            result.metadata16 = Some(metadata);
            let metadata = metadata52::Catalog::from_rom(rom)?;
            metadata.decoded.validate_links(&result.item_links.as_ref().unwrap().decoded)?;
            result.manifest.coverage.push(METADATA52_ID.into());
            result.manifest.resources.push(Resource {
                id: METADATA52_ID.into(), path: METADATA52_FILE.into(), format: METADATA52_FORMAT.into(),
                sha256: String::new(), dependencies: vec![ITEM_LINKS_ID.into()], sources: metadata.sources.clone(),
            });
            result.metadata52 = Some(metadata);
            let metadata = metadata4::Catalog::from_rom(rom)?;
            metadata.decoded.validate_metadata52(&result.metadata52.as_ref().unwrap().decoded)?;
            result.manifest.coverage.push(METADATA4_ID.into());
            result.manifest.resources.push(Resource {
                id: METADATA4_ID.into(), path: METADATA4_FILE.into(), format: METADATA4_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA52_ID.into()], sources: metadata.sources.clone(),
            });
            result.metadata4 = Some(metadata);
            let metadata = metadata2::Catalog::from_rom(rom)?;
            metadata.decoded.validate_metadata52(&result.metadata52.as_ref().unwrap().decoded)?;
            result.manifest.coverage.push(METADATA2_ID.into());
            result.manifest.resources.push(Resource {
                id: METADATA2_ID.into(), path: METADATA2_FILE.into(), format: METADATA2_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA52_ID.into()], sources: metadata.sources.clone(),
            });
            result.metadata2 = Some(metadata);
            let metadata = default48::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(DEFAULT48_ID.into());
            result.manifest.resources.push(Resource {
                id: DEFAULT48_ID.into(), path: DEFAULT48_FILE.into(), format: DEFAULT48_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.default48 = Some(metadata);
            let metadata = constructor_alternates::Catalog::from_rom(rom)?;
            metadata.decoded.validate_items(&result.items.as_ref().unwrap().decoded)?;
            result.manifest.coverage.push(CONSTRUCTOR_ALTERNATES_ID.into());
            result.manifest.resources.push(Resource {
                id: CONSTRUCTOR_ALTERNATES_ID.into(), path: CONSTRUCTOR_ALTERNATES_FILE.into(), format: CONSTRUCTOR_ALTERNATES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![ITEM_RECORDS_ID.into()], sources: metadata.sources.clone(),
            });
            result.constructor_alternates = Some(metadata);
            let metadata = metadata14::Catalog::from_rom(rom)?;
            metadata.decoded.validate_metadata52(&result.metadata52.as_ref().unwrap().decoded)?;
            result.manifest.coverage.push(METADATA14_ID.into());
            result.manifest.resources.push(Resource {
                id: METADATA14_ID.into(), path: METADATA14_FILE.into(), format: METADATA14_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA52_ID.into()], sources: metadata.sources.clone(),
            });
            result.metadata14 = Some(metadata);
            let metadata = metadata12::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(METADATA12_ID.into());
            result.manifest.resources.push(Resource {
                id: METADATA12_ID.into(), path: METADATA12_FILE.into(), format: METADATA12_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.metadata12 = Some(metadata);
            let metadata = global_pair_flags::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(GLOBAL_PAIR_FLAGS_ID.into());
            result.manifest.resources.push(Resource {
                id: GLOBAL_PAIR_FLAGS_ID.into(), path: GLOBAL_PAIR_FLAGS_FILE.into(), format: GLOBAL_PAIR_FLAGS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.global_pair_flags = Some(metadata);
            let metadata = evaluation_parameters::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(EVALUATION_PARAMETERS_ID.into());
            result.manifest.resources.push(Resource {
                id: EVALUATION_PARAMETERS_ID.into(), path: EVALUATION_PARAMETERS_FILE.into(), format: EVALUATION_PARAMETERS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.evaluation_parameters = Some(metadata);
            let metadata = progression_rewards::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(PROGRESSION_REWARDS_ID.into());
            result.manifest.resources.push(Resource {
                id: PROGRESSION_REWARDS_ID.into(), path: PROGRESSION_REWARDS_FILE.into(), format: PROGRESSION_REWARDS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.progression_rewards = Some(metadata);
            let metadata = progression_gates::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(PROGRESSION_GATES_ID.into());
            result.manifest.resources.push(Resource {
                id: PROGRESSION_GATES_ID.into(), path: PROGRESSION_GATES_FILE.into(), format: PROGRESSION_GATES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.progression_gates = Some(metadata);
            let metadata = progression_selection_gates::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(PROGRESSION_SELECTION_GATES_ID.into());
            result.manifest.resources.push(Resource {
                id: PROGRESSION_SELECTION_GATES_ID.into(), path: PROGRESSION_SELECTION_GATES_FILE.into(), format: PROGRESSION_SELECTION_GATES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.progression_selection_gates = Some(metadata);
            let metadata = aggregate_flag_keys::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(AGGREGATE_FLAG_KEYS_ID.into());
            result.manifest.resources.push(Resource {
                id: AGGREGATE_FLAG_KEYS_ID.into(), path: AGGREGATE_FLAG_KEYS_FILE.into(), format: AGGREGATE_FLAG_KEYS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.aggregate_flag_keys = Some(metadata);
            let metadata = trade_parameters::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(TRADE_PARAMETERS_ID.into());
            result.manifest.resources.push(Resource {
                id: TRADE_PARAMETERS_ID.into(), path: TRADE_PARAMETERS_FILE.into(), format: TRADE_PARAMETERS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.trade_parameters = Some(metadata);
            let metadata = item_effect_parameters::Catalog::from_rom(rom)?;
            metadata.decoded.validate_items(&result.items.as_ref().unwrap().decoded)?;
            result.manifest.coverage.push(ITEM_EFFECT_PARAMETERS_ID.into());
            result.manifest.resources.push(Resource {
                id: ITEM_EFFECT_PARAMETERS_ID.into(), path: ITEM_EFFECT_PARAMETERS_FILE.into(), format: ITEM_EFFECT_PARAMETERS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![ITEM_RECORDS_ID.into()], sources: metadata.sources.clone(),
            });
            result.item_effect_parameters = Some(metadata);
            let metadata = audio_directory::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(AUDIO_DIRECTORY_ID.into());
            result.manifest.resources.push(Resource {
                id: AUDIO_DIRECTORY_ID.into(), path: AUDIO_DIRECTORY_FILE.into(), format: AUDIO_DIRECTORY_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.audio_directory = Some(metadata);
            let metadata = audio_tones::Catalog::from_rom(rom)?;
            metadata.decoded.validate_directory(&result.audio_directory.as_ref().unwrap().decoded)?;
            result.manifest.coverage.push(AUDIO_TONES_ID.into());
            result.manifest.resources.push(Resource {
                id: AUDIO_TONES_ID.into(), path: AUDIO_TONES_FILE.into(), format: AUDIO_TONES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![AUDIO_DIRECTORY_ID.into()], sources: metadata.sources.clone(),
            });
            result.audio_tones = Some(metadata);
            let metadata = audio_sequences::Catalog::from_rom(rom)?;
            metadata.decoded.validate_owners(&result.audio_directory.as_ref().unwrap().decoded,&result.audio_tones.as_ref().unwrap().decoded)?;
            result.manifest.coverage.push(AUDIO_SEQUENCES_ID.into());
            result.manifest.resources.push(Resource {
                id: AUDIO_SEQUENCES_ID.into(), path: AUDIO_SEQUENCES_FILE.into(), format: AUDIO_SEQUENCES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![AUDIO_DIRECTORY_ID.into(),AUDIO_TONES_ID.into()], sources: metadata.sources.clone(),
            });
            result.audio_sequences = Some(metadata);
            let metadata = audio_driver::Catalog::from_rom(rom)?;
            metadata.decoded.validate_sequence_owner(&result.audio_sequences.as_ref().unwrap().decoded)?;
            result.manifest.coverage.push(AUDIO_DRIVER_ID.into());
            result.manifest.resources.push(Resource {
                id: AUDIO_DRIVER_ID.into(), path: AUDIO_DRIVER_FILE.into(), format: AUDIO_DRIVER_FORMAT.into(),
                sha256: String::new(), dependencies: vec![AUDIO_SEQUENCES_ID.into()], sources: metadata.sources.clone(),
            });
            result.audio_driver = Some(metadata);
            let metadata = palette_effect_dispatch::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(PALETTE_EFFECT_DISPATCH_ID.into());
            result.manifest.resources.push(Resource {
                id: PALETTE_EFFECT_DISPATCH_ID.into(), path: PALETTE_EFFECT_DISPATCH_FILE.into(), format: PALETTE_EFFECT_DISPATCH_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.palette_effect_dispatch = Some(metadata);
            let metadata = palette_programs::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(PALETTE_PROGRAMS_ID.into());
            result.manifest.resources.push(Resource {
                id: PALETTE_PROGRAMS_ID.into(), path: PALETTE_PROGRAMS_FILE.into(), format: PALETTE_PROGRAMS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.palette_programs = Some(metadata);
            let metadata = blend_programs::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(BLEND_PROGRAMS_ID.into());
            result.manifest.resources.push(Resource {
                id: BLEND_PROGRAMS_ID.into(), path: BLEND_PROGRAMS_FILE.into(), format: BLEND_PROGRAMS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.blend_programs = Some(metadata);
            let metadata = task_descriptors::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(TASK_DESCRIPTORS_ID.into());
            result.manifest.resources.push(Resource {
                id: TASK_DESCRIPTORS_ID.into(), path: TASK_DESCRIPTORS_FILE.into(), format: TASK_DESCRIPTORS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.task_descriptors = Some(metadata);
            let metadata = effect_event_directories::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(EFFECT_EVENT_DIRECTORIES_ID.into());
            result.manifest.resources.push(Resource {
                id: EFFECT_EVENT_DIRECTORIES_ID.into(), path: EFFECT_EVENT_DIRECTORIES_FILE.into(), format: EFFECT_EVENT_DIRECTORIES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![TASK_DESCRIPTORS_ID.into()], sources: metadata.sources.clone(),
            });
            result.effect_event_directories = Some(metadata);
            let metadata = script_commands::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(SCRIPT_COMMANDS_ID.into());
            result.manifest.resources.push(Resource {
                id: SCRIPT_COMMANDS_ID.into(), path: SCRIPT_COMMANDS_FILE.into(), format: SCRIPT_COMMANDS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.script_commands = Some(metadata);
            let metadata = script_archive::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(SCRIPT_ARCHIVE_ID.into());
            result.manifest.resources.push(Resource {
                id: SCRIPT_ARCHIVE_ID.into(), path: SCRIPT_ARCHIVE_FILE.into(), format: SCRIPT_ARCHIVE_FORMAT.into(),
                sha256: String::new(), dependencies: vec![SCRIPT_COMMANDS_ID.into(), EFFECT_EVENT_DIRECTORIES_ID.into()], sources: metadata.sources.clone(),
            });
            result.script_archive = Some(metadata);
            let metadata = script_text::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(SCRIPT_TEXT_ID.into());
            result.manifest.resources.push(Resource {
                id: SCRIPT_TEXT_ID.into(), path: SCRIPT_TEXT_FILE.into(), format: SCRIPT_TEXT_FORMAT.into(),
                sha256: String::new(), dependencies: vec![SCRIPT_ARCHIVE_ID.into(), FONT_BANK_ID.into()], sources: metadata.sources.clone(),
            });
            result.script_text = Some(metadata);
            let metadata = script_coordinates::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(SCRIPT_COORDINATES_ID.into());
            result.manifest.resources.push(Resource {
                id: SCRIPT_COORDINATES_ID.into(), path: SCRIPT_COORDINATES_FILE.into(), format: SCRIPT_COORDINATES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![SCRIPT_ARCHIVE_ID.into(), SCRIPT_COMMANDS_ID.into()], sources: metadata.sources.clone(),
            });
            result.script_coordinates = Some(metadata);
            let metadata = auxiliary_scripts::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(AUXILIARY_COMMANDS_ID.into());
            result.manifest.resources.push(Resource {
                id: AUXILIARY_COMMANDS_ID.into(), path: AUXILIARY_COMMANDS_FILE.into(), format: AUXILIARY_COMMANDS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: vec![metadata.sources[1].clone()],
            });
            result.manifest.coverage.push(AUXILIARY_ARCHIVE_ID.into());
            result.manifest.resources.push(Resource {
                id: AUXILIARY_ARCHIVE_ID.into(), path: AUXILIARY_ARCHIVE_FILE.into(), format: AUXILIARY_ARCHIVE_FORMAT.into(),
                sha256: String::new(), dependencies: vec![AUXILIARY_COMMANDS_ID.into()], sources: vec![metadata.sources[0].clone()],
            });
            result.auxiliary_scripts = Some(metadata);
            let metadata = map_raster_palettes::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(MAP_RASTER_PALETTES_ID.into());
            result.manifest.resources.push(Resource {
                id: MAP_RASTER_PALETTES_ID.into(), path: MAP_RASTER_PALETTES_FILE.into(), format: MAP_RASTER_PALETTES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![BG_DESCRIPTORS_ID.into()], sources: metadata.sources.clone(),
            });
            result.map_raster_palettes = Some(metadata);
            let metadata = ui_choice_lists::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UI_CHOICE_LISTS_ID.into());
            result.manifest.resources.push(Resource {
                id: UI_CHOICE_LISTS_ID.into(), path: UI_CHOICE_LISTS_FILE.into(), format: UI_CHOICE_LISTS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.ui_choice_lists = Some(metadata);
            let metadata = unit_obj_initial_pose::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_INITIAL_POSE_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_INITIAL_POSE_ID.into(), path: UNIT_OBJ_INITIAL_POSE_FILE.into(), format: UNIT_OBJ_INITIAL_POSE_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA52_ID.into(), METADATA14_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_initial_pose = Some(metadata);
            let metadata = color_mode_palettes::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(COLOR_MODE_PALETTES_ID.into());
            result.manifest.resources.push(Resource {
                id: COLOR_MODE_PALETTES_ID.into(), path: COLOR_MODE_PALETTES_FILE.into(), format: COLOR_MODE_PALETTES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA14_ID.into()], sources: metadata.sources.clone(),
            });
            result.color_mode_palettes = Some(metadata);
            let metadata = unit_obj_equipment::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_EQUIPMENT_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_EQUIPMENT_ID.into(), path: UNIT_OBJ_EQUIPMENT_FILE.into(), format: UNIT_OBJ_EQUIPMENT_FORMAT.into(),
                sha256: String::new(), dependencies: vec![ITEM_RECORDS_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_equipment = Some(metadata);
            let metadata = unit_obj_normal_battle::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_NORMAL_BATTLE_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_NORMAL_BATTLE_ID.into(), path: UNIT_OBJ_NORMAL_BATTLE_FILE.into(), format: UNIT_OBJ_NORMAL_BATTLE_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA52_ID.into(), METADATA14_ID.into(), UNIT_OBJ_INITIAL_POSE_ID.into(), UNIT_OBJ_EQUIPMENT_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_normal_battle = Some(metadata);
            let metadata = unit_obj_alternate_battle::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_ALTERNATE_BATTLE_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_ALTERNATE_BATTLE_ID.into(), path: UNIT_OBJ_ALTERNATE_BATTLE_FILE.into(), format: UNIT_OBJ_ALTERNATE_BATTLE_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA52_ID.into(), METADATA14_ID.into(), UNIT_OBJ_INITIAL_POSE_ID.into(), UNIT_OBJ_EQUIPMENT_ID.into(), UNIT_OBJ_NORMAL_BATTLE_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_alternate_battle = Some(metadata);
            let metadata = unit_obj_script_special_normal::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_ID.into(), path: UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FILE.into(), format: UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA14_ID.into(), SCRIPT_ARCHIVE_ID.into(), UNIT_OBJ_INITIAL_POSE_ID.into(), UNIT_OBJ_NORMAL_BATTLE_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_script_special_normal = Some(metadata);
            let metadata = unit_obj_script_special_alternate::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_ID.into(), path: UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FILE.into(), format: UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA14_ID.into(), SCRIPT_ARCHIVE_ID.into(), UNIT_OBJ_INITIAL_POSE_ID.into(), UNIT_OBJ_NORMAL_BATTLE_ID.into(), UNIT_OBJ_ALTERNATE_BATTLE_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_script_special_alternate = Some(metadata);
            let metadata = unit_obj_script_override::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_SCRIPT_OVERRIDE_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_SCRIPT_OVERRIDE_ID.into(), path: UNIT_OBJ_SCRIPT_OVERRIDE_FILE.into(), format: UNIT_OBJ_SCRIPT_OVERRIDE_FORMAT.into(),
                sha256: String::new(), dependencies: vec![SCRIPT_ARCHIVE_ID.into(), UNIT_OBJ_NORMAL_BATTLE_ID.into(), UNIT_OBJ_ALTERNATE_BATTLE_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_script_override = Some(metadata);
            let metadata = unit_obj_script_registry_special::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_ID.into(), path: UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FILE.into(), format: UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA14_ID.into(), SCRIPT_ARCHIVE_ID.into(), UNIT_OBJ_INITIAL_POSE_ID.into(), UNIT_OBJ_EQUIPMENT_ID.into(), UNIT_OBJ_NORMAL_BATTLE_ID.into(), UNIT_OBJ_ALTERNATE_BATTLE_ID.into(), UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_ID.into(), UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_script_registry_special = Some(metadata);
            let metadata = unit_obj_script_registry_generic::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_ID.into(), path: UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FILE.into(), format: UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA52_ID.into(), SCRIPT_ARCHIVE_ID.into(), UNIT_OBJ_INITIAL_POSE_ID.into(), UNIT_OBJ_EQUIPMENT_ID.into(), UNIT_OBJ_NORMAL_BATTLE_ID.into(), UNIT_OBJ_ALTERNATE_BATTLE_ID.into(), UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_ID.into(), UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_script_registry_generic = Some(metadata);
            let metadata = unit_obj_property_words::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_PROPERTY_WORDS_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_PROPERTY_WORDS_ID.into(), path: UNIT_OBJ_PROPERTY_WORDS_FILE.into(), format: UNIT_OBJ_PROPERTY_WORDS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA52_ID.into(), METADATA14_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_property_words = Some(metadata);
            let metadata = unit_obj_byte_property_words::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_BYTE_PROPERTY_WORDS_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_BYTE_PROPERTY_WORDS_ID.into(), path: UNIT_OBJ_BYTE_PROPERTY_WORDS_FILE.into(), format: UNIT_OBJ_BYTE_PROPERTY_WORDS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![UNIT_OBJ_PROPERTY_WORDS_ID.into(), COLOR_MODE_PALETTES_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_byte_property_words = Some(metadata);
            let metadata = unit_obj_ffff_reader_fields::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UNIT_OBJ_FFFF_READER_FIELDS_ID.into());
            result.manifest.resources.push(Resource {
                id: UNIT_OBJ_FFFF_READER_FIELDS_ID.into(), path: UNIT_OBJ_FFFF_READER_FIELDS_FILE.into(), format: UNIT_OBJ_FFFF_READER_FIELDS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA52_ID.into(), SCRIPT_ARCHIVE_ID.into()], sources: metadata.sources.clone(),
            });
            result.unit_obj_ffff_reader_fields = Some(metadata);
            let metadata = slot_job_lookup::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(SLOT_JOB_LOOKUP_ID.into());
            result.manifest.resources.push(Resource {
                id: SLOT_JOB_LOOKUP_ID.into(), path: SLOT_JOB_LOOKUP_FILE.into(), format: SLOT_JOB_LOOKUP_FORMAT.into(),
                sha256: String::new(), dependencies: vec![METADATA52_ID.into()], sources: metadata.sources.clone(),
            });
            result.slot_job_lookup = Some(metadata);
            let metadata = observed_task_parameters::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(OBSERVED_TASK_PARAMETERS_ID.into());
            result.manifest.resources.push(Resource {
                id: OBSERVED_TASK_PARAMETERS_ID.into(), path: OBSERVED_TASK_PARAMETERS_FILE.into(), format: OBSERVED_TASK_PARAMETERS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.observed_task_parameters = Some(metadata);
            let mut metadata = obj_lists::constructor_animation::Catalog::from_rom(rom)?;
            metadata.sources.retain(|s| s.role == "constructor-obj-animation");
            result.manifest.coverage.push(CONSTRUCTOR_OBJ_ANIMATION_ID.into());
            result.manifest.resources.push(Resource {
                id: CONSTRUCTOR_OBJ_ANIMATION_ID.into(), path: CONSTRUCTOR_OBJ_ANIMATION_FILE.into(), format: CONSTRUCTOR_OBJ_ANIMATION_FORMAT.into(),
                sha256: String::new(), dependencies: vec![OBJ_STATIC_LISTS_ID.into()], sources: metadata.sources.clone(),
            });
            result.constructor_obj_animation = Some(metadata);
            let metadata = task_cycle_deltas::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(TASK_CYCLE_DELTAS_ID.into());
            result.manifest.resources.push(Resource {
                id: TASK_CYCLE_DELTAS_ID.into(), path: TASK_CYCLE_DELTAS_FILE.into(), format: TASK_CYCLE_DELTAS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.task_cycle_deltas = Some(metadata);
            let metadata = word_task_obj_animations::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(WORD_TASK_OBJ_ANIMATIONS_ID.into());
            result.manifest.resources.push(Resource {
                id: WORD_TASK_OBJ_ANIMATIONS_ID.into(), path: WORD_TASK_OBJ_ANIMATIONS_FILE.into(), format: WORD_TASK_OBJ_ANIMATIONS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.word_task_obj_animations = Some(metadata);
            let metadata = world_nodes::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(WORLD_NODES_ID.into());
            result.manifest.resources.push(Resource {
                id: WORLD_NODES_ID.into(), path: WORLD_NODES_FILE.into(), format: WORLD_NODES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.world_nodes = Some(metadata);
            let metadata = world_roads::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(WORLD_ROADS_ID.into());
            result.manifest.resources.push(Resource {
                id: WORLD_ROADS_ID.into(), path: WORLD_ROADS_FILE.into(), format: WORLD_ROADS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![], sources: metadata.sources.clone(),
            });
            result.world_roads = Some(metadata);
            let metadata = encounter_headers::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(ENCOUNTER_HEADERS_ID.into());
            result.manifest.resources.push(Resource {
                id: ENCOUNTER_HEADERS_ID.into(), path: ENCOUNTER_HEADERS_FILE.into(), format: ENCOUNTER_HEADERS_FORMAT.into(),
                sha256: String::new(), dependencies: vec![CONSTRUCTOR_ALTERNATES_ID.into()], sources: metadata.sources.clone(),
            });
            result.encounter_headers = Some(metadata);
            let metadata = encounter_unit_templates::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(ENCOUNTER_UNIT_TEMPLATES_ID.into());
            result.manifest.resources.push(Resource {
                id: ENCOUNTER_UNIT_TEMPLATES_ID.into(), path: ENCOUNTER_UNIT_TEMPLATES_FILE.into(), format: ENCOUNTER_UNIT_TEMPLATES_FORMAT.into(),
                sha256: String::new(), dependencies: vec![ENCOUNTER_HEADERS_ID.into(), CONSTRUCTOR_ALTERNATES_ID.into(), ITEM_RECORDS_ID.into(), METADATA14_ID.into(), METADATA52_ID.into()], sources: metadata.sources.clone(),
            });
            result.encounter_unit_templates = Some(metadata);
            let metadata = encounter_coordinates::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(ENCOUNTER_COORDINATES_ID.into());
            result.manifest.resources.push(Resource {id:ENCOUNTER_COORDINATES_ID.into(),path:ENCOUNTER_COORDINATES_FILE.into(),format:ENCOUNTER_COORDINATES_FORMAT.into(),sha256:String::new(),dependencies:vec![ENCOUNTER_HEADERS_ID.into()],sources:metadata.sources.clone()});
            result.encounter_coordinates = Some(metadata);
            let metadata = metadata28::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(METADATA28_ID.into());
            result.manifest.resources.push(Resource {id:METADATA28_ID.into(),path:METADATA28_FILE.into(),format:METADATA28_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.metadata28 = Some(metadata);
            let metadata = records4::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(RECORDS4_ID.into());
            result.manifest.resources.push(Resource {id:RECORDS4_ID.into(),path:RECORDS4_FILE.into(),format:RECORDS4_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.records4 = Some(metadata);
            let metadata = conditional_records8::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(CONDITIONAL_RECORDS8_ID.into());
            result.manifest.resources.push(Resource {id:CONDITIONAL_RECORDS8_ID.into(),path:CONDITIONAL_RECORDS8_FILE.into(),format:CONDITIONAL_RECORDS8_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.conditional_records8 = Some(metadata);
            let metadata = mission_records70::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(MISSION_RECORDS70_ID.into());
            result.manifest.resources.push(Resource {id:MISSION_RECORDS70_ID.into(),path:MISSION_RECORDS70_FILE.into(),format:MISSION_RECORDS70_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.mission_records70 = Some(metadata);
            let metadata = selection_byte_tables::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(SELECTION_BYTE_TABLES_ID.into());
            result.manifest.resources.push(Resource {id:SELECTION_BYTE_TABLES_ID.into(),path:SELECTION_BYTE_TABLES_FILE.into(),format:SELECTION_BYTE_TABLES_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.selection_byte_tables = Some(metadata);
            let metadata = event_records12::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(EVENT_RECORDS12_ID.into());
            result.manifest.resources.push(Resource {id:EVENT_RECORDS12_ID.into(),path:EVENT_RECORDS12_FILE.into(),format:EVENT_RECORDS12_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.event_records12 = Some(metadata);
            let metadata = parameter_records27::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(PARAMETER_RECORDS27_ID.into());
            result.manifest.resources.push(Resource {id:PARAMETER_RECORDS27_ID.into(),path:PARAMETER_RECORDS27_FILE.into(),format:PARAMETER_RECORDS27_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.parameter_records27 = Some(metadata);
            let metadata = world_records38::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(WORLD_RECORDS38_ID.into());
            result.manifest.resources.push(Resource {id:WORLD_RECORDS38_ID.into(),path:WORLD_RECORDS38_FILE.into(),format:WORLD_RECORDS38_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.world_records38 = Some(metadata);
            let metadata = indexed_graphics::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(INDEXED_GRAPHICS_ID.into());
            result.manifest.resources.push(Resource {id:INDEXED_GRAPHICS_ID.into(),path:INDEXED_GRAPHICS_FILE.into(),format:INDEXED_GRAPHICS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.indexed_graphics = Some(metadata);
            let metadata = ui_raw_transfers::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(UI_RAW_TRANSFERS_ID.into());
            result.manifest.resources.push(Resource {id:UI_RAW_TRANSFERS_ID.into(),path:UI_RAW_TRANSFERS_FILE.into(),format:UI_RAW_TRANSFERS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.ui_raw_transfers = Some(metadata);
            let metadata = ui_raw_transfers::dynamic_fragments::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(DYNAMIC_RAW_FRAGMENTS_ID.into());
            result.manifest.resources.push(Resource {id:DYNAMIC_RAW_FRAGMENTS_ID.into(),path:DYNAMIC_RAW_FRAGMENTS_FILE.into(),format:DYNAMIC_RAW_FRAGMENTS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.dynamic_raw_fragments = Some(metadata);
            let metadata = conditional_four_byte_lookup::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(CONDITIONAL_FOUR_BYTE_LOOKUP_ID.into());
            result.manifest.resources.push(Resource {id:CONDITIONAL_FOUR_BYTE_LOOKUP_ID.into(),path:CONDITIONAL_FOUR_BYTE_LOOKUP_FILE.into(),format:CONDITIONAL_FOUR_BYTE_LOOKUP_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.conditional_four_byte_lookup = Some(metadata);
            let metadata = conditional_packed_lookup::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(CONDITIONAL_PACKED_LOOKUP_ID.into());
            result.manifest.resources.push(Resource {id:CONDITIONAL_PACKED_LOOKUP_ID.into(),path:CONDITIONAL_PACKED_LOOKUP_FILE.into(),format:CONDITIONAL_PACKED_LOOKUP_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.conditional_packed_lookup = Some(metadata);
            let metadata = conditional_descriptor_banks::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(CONDITIONAL_DESCRIPTOR_BANKS_ID.into());
            result.manifest.resources.push(Resource {id:CONDITIONAL_DESCRIPTOR_BANKS_ID.into(),path:CONDITIONAL_DESCRIPTOR_BANKS_FILE.into(),format:CONDITIONAL_DESCRIPTOR_BANKS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.conditional_descriptor_banks = Some(metadata);
            let metadata = conditional_descriptor_bank_zero::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(CONDITIONAL_DESCRIPTOR_BANK_ZERO_ID.into());
            result.manifest.resources.push(Resource {id:CONDITIONAL_DESCRIPTOR_BANK_ZERO_ID.into(),path:CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE.into(),format:CONDITIONAL_DESCRIPTOR_BANK_ZERO_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.conditional_descriptor_bank_zero = Some(metadata);
            let metadata = conditional_property_masks::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(CONDITIONAL_PROPERTY_MASKS_ID.into());
            result.manifest.resources.push(Resource {id:CONDITIONAL_PROPERTY_MASKS_ID.into(),path:CONDITIONAL_PROPERTY_MASKS_FILE.into(),format:CONDITIONAL_PROPERTY_MASKS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.conditional_property_masks = Some(metadata);
            let metadata = compact_obj_animations::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(COMPACT_OBJ_ANIMATIONS_ID.into());
            result.manifest.resources.push(Resource {id:COMPACT_OBJ_ANIMATIONS_ID.into(),path:COMPACT_OBJ_ANIMATIONS_FILE.into(),format:COMPACT_OBJ_ANIMATIONS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.compact_obj_animations = Some(metadata);
            let metadata = compact_obj_animations_secondary::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(SECONDARY_COMPACT_OBJ_ANIMATIONS_ID.into());
            result.manifest.resources.push(Resource {id:SECONDARY_COMPACT_OBJ_ANIMATIONS_ID.into(),path:SECONDARY_COMPACT_OBJ_ANIMATIONS_FILE.into(),format:SECONDARY_COMPACT_OBJ_ANIMATIONS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.secondary_compact_obj_animations = Some(metadata);
            let metadata = compact_obj_animations_tertiary::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(TERTIARY_COMPACT_OBJ_ANIMATIONS_ID.into());
            result.manifest.resources.push(Resource {id:TERTIARY_COMPACT_OBJ_ANIMATIONS_ID.into(),path:TERTIARY_COMPACT_OBJ_ANIMATIONS_FILE.into(),format:TERTIARY_COMPACT_OBJ_ANIMATIONS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.tertiary_compact_obj_animations = Some(metadata);
            let metadata = compact_obj_animations_quaternary::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(QUATERNARY_COMPACT_OBJ_ANIMATIONS_ID.into());
            result.manifest.resources.push(Resource {id:QUATERNARY_COMPACT_OBJ_ANIMATIONS_ID.into(),path:QUATERNARY_COMPACT_OBJ_ANIMATIONS_FILE.into(),format:QUATERNARY_COMPACT_OBJ_ANIMATIONS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.quaternary_compact_obj_animations = Some(metadata);
            let metadata = compact_obj_animations_quinary::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(QUINARY_COMPACT_OBJ_ANIMATIONS_ID.into());
            result.manifest.resources.push(Resource {id:QUINARY_COMPACT_OBJ_ANIMATIONS_ID.into(),path:QUINARY_COMPACT_OBJ_ANIMATIONS_FILE.into(),format:QUINARY_COMPACT_OBJ_ANIMATIONS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.quinary_compact_obj_animations = Some(metadata);
            let metadata = compact_obj_animations_senary::Catalog::from_rom(rom)?;
            result.manifest.coverage.push(SENARY_COMPACT_OBJ_ANIMATIONS_ID.into());
            result.manifest.resources.push(Resource {id:SENARY_COMPACT_OBJ_ANIMATIONS_ID.into(),path:SENARY_COMPACT_OBJ_ANIMATIONS_FILE.into(),format:SENARY_COMPACT_OBJ_ANIMATIONS_FORMAT.into(),sha256:String::new(),dependencies:vec![],sources:metadata.sources.clone()});
            result.senary_compact_obj_animations = Some(metadata);
        } else {
            result.manifest.extractor_revision = 8;
            result.manifest.decoder_revision = 7;
        }
        Ok(result)
    }

    fn from_ranges(
        rom: &[u8],
        hash: String,
        profile: &str,
        ranges: &[(u32, &str)],
        expected: &str,
    ) -> Result<Self, String> {
        let mut sources = Vec::new();
        let mut dimensions = None;
        for &(offset, role) in ranges {
            let start = offset as usize;
            let bytes = rom
                .get(start..start.checked_add(48).ok_or("Source range overflow")?)
                .ok_or("Truncated OBJ dimensions source range")?;
            let sha256 = ffta_rom::digest(bytes);
            if sha256 != expected {
                return Err(format!("OBJ dimensions source mismatch at {offset:#x}"));
            }
            let decoded = ObjDimensions::decode(bytes)?;
            if dimensions.as_ref().is_some_and(|d| d != &decoded) {
                return Err("OBJ dimensions source aliases disagree".into());
            }
            dimensions = Some(decoded);
            sources.push(SourceRange {
                offset,
                length: 48,
                sha256,
                role: role.into(),
            });
        }
        Ok(Self {
            dimensions: dimensions.ok_or("Missing OBJ dimensions source")?,
            background: None,
            palettes: None,
            alternate_palettes: None,
            packet_programs: None,
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
            manifest: Manifest {
                schema: SCHEMA,
                extractor_revision: EXTRACTOR_REV,
                decoder_revision: DECODER_REV,
                source_rom_sha256: hash,
                source_rom_size: u32::try_from(rom.len()).map_err(|_| "ROM is too large")?,
                profile: profile.into(),
                coverage: vec![OBJ_DIMENSIONS_ID.into()],
                resources: vec![Resource {
                    id: OBJ_DIMENSIONS_ID.into(),
                    path: OBJ_DIMENSIONS_FILE.into(),
                    format: "obj-dimensions-v1".into(),
                    // File bytes are defined by the frontend serializer.
                    sha256: String::new(),
                    dependencies: Vec::new(),
                    sources,
                }],
            },
        })
    }

    pub fn set_file_hashes(&mut self, files: &[&[u8]]) -> Result<(), String> {
        if self.manifest.resources.len()
            != self.resource_count()
            || files.len() != self.manifest.resources.len()
        {
            return Err("Resource set does not match extraction".into());
        }
        for (resource, bytes) in self.manifest.resources.iter_mut().zip(files) {
            resource.sha256 = ffta_rom::digest(bytes);
        }
        Ok(())
    }

    /// The expected manifest comes from the selected source, never the files.
    /// A self-consistent edited resource is still an invalid *base* dataset.
    pub fn validate_files(&self, files: BaseFiles<'_>) -> Result<(), String> {
        if self.palettes.is_some() && self.background.is_none()
            || self.alternate_palettes.is_some() && self.palettes.is_none()
            || self.packet_programs.is_some() && self.alternate_palettes.is_none()
            || self.main_graphics.is_some() && self.packet_programs.is_none()
            || self.transfers.is_some() && self.main_graphics.is_none()
            || self.short_texts.is_some() && self.transfers.is_none()
            || self.text_catalogs.is_some() && self.short_texts.is_none()
            || self.font_bank.is_some() && self.text_catalogs.is_none()
            || self.name_keyboard.is_some() && self.font_bank.is_none()
            || self.obj_lists.is_some() && self.name_keyboard.is_none()
            || self.obj_animations.is_some() && self.obj_lists.is_none()
            || self.obj_graphics.is_some() && self.obj_animations.is_none()
            || self.ui_palettes.is_some() && self.obj_graphics.is_none()
            || self.ui_graphics.is_some() && self.ui_palettes.is_none()
            || self.items.is_some() && self.ui_graphics.is_none()
            || self.item_links.is_some() && self.items.is_none()
            || self.metadata16.is_some() && self.item_links.is_none()
            || self.metadata52.is_some() && self.metadata16.is_none()
            || self.metadata4.is_some() && self.metadata52.is_none()
            || self.metadata2.is_some() && self.metadata52.is_none()
            || self.default48.is_some() && self.metadata2.is_none()
            || self.constructor_alternates.is_some() && self.default48.is_none()
            || self.metadata14.is_some() && self.constructor_alternates.is_none()
            || self.metadata12.is_some() && self.metadata14.is_none()
            || self.global_pair_flags.is_some() && self.metadata12.is_none()
            || self.evaluation_parameters.is_some() && self.global_pair_flags.is_none()
            || self.progression_rewards.is_some() && self.evaluation_parameters.is_none()
            || self.progression_gates.is_some() && self.progression_rewards.is_none()
            || self.progression_selection_gates.is_some() && self.progression_gates.is_none()
            || self.aggregate_flag_keys.is_some() && self.progression_selection_gates.is_none()
            || self.trade_parameters.is_some() && self.aggregate_flag_keys.is_none()
            || self.item_effect_parameters.is_some() && self.trade_parameters.is_none()
            || self.audio_directory.is_some() && self.item_effect_parameters.is_none()
            || self.audio_tones.is_some() && self.audio_directory.is_none()
            || self.audio_sequences.is_some() && self.audio_tones.is_none()
            || self.audio_driver.is_some() && self.audio_sequences.is_none()
            || self.palette_effect_dispatch.is_some() && self.audio_driver.is_none()
            || self.palette_programs.is_some() && self.palette_effect_dispatch.is_none()
            || self.blend_programs.is_some() && self.palette_programs.is_none()
            || self.task_descriptors.is_some() && self.blend_programs.is_none()
            || self.effect_event_directories.is_some() && self.task_descriptors.is_none()
            || self.script_commands.is_some() && self.effect_event_directories.is_none()
            || self.script_archive.is_some() && self.script_commands.is_none()
            || self.script_text.is_some() && self.script_archive.is_none()
            || self.script_coordinates.is_some() && self.script_archive.is_none()
            || self.ui_choice_lists.is_some() && self.map_raster_palettes.is_none()
            || self.unit_obj_initial_pose.is_some() && (self.ui_choice_lists.is_none() || self.metadata52.is_none() || self.metadata14.is_none())
            || self.color_mode_palettes.is_some() && (self.unit_obj_initial_pose.is_none() || self.metadata14.is_none())
            || self.unit_obj_equipment.is_some() && (self.color_mode_palettes.is_none() || self.items.is_none())
            || self.unit_obj_normal_battle.is_some() && (self.unit_obj_equipment.is_none() || self.unit_obj_initial_pose.is_none() || self.metadata52.is_none() || self.metadata14.is_none())
            || self.unit_obj_alternate_battle.is_some() && (self.unit_obj_normal_battle.is_none() || self.unit_obj_equipment.is_none() || self.unit_obj_initial_pose.is_none() || self.metadata52.is_none() || self.metadata14.is_none())
            || self.unit_obj_script_special_normal.is_some() && (self.unit_obj_alternate_battle.is_none() || self.script_archive.is_none() || self.metadata14.is_none() || self.unit_obj_initial_pose.is_none() || self.unit_obj_normal_battle.is_none())
            || self.unit_obj_script_special_alternate.is_some() && (self.unit_obj_alternate_battle.is_none() || self.script_archive.is_none() || self.metadata14.is_none() || self.unit_obj_initial_pose.is_none() || self.unit_obj_normal_battle.is_none())
            || self.unit_obj_script_override.is_some() && (self.unit_obj_alternate_battle.is_none() || self.script_archive.is_none() || self.unit_obj_normal_battle.is_none())
            || self.unit_obj_script_registry_special.is_some() && (self.metadata14.is_none() || self.script_archive.is_none() || self.unit_obj_initial_pose.is_none() || self.unit_obj_equipment.is_none() || self.unit_obj_normal_battle.is_none() || self.unit_obj_alternate_battle.is_none() || self.unit_obj_script_special_normal.is_none() || self.unit_obj_script_special_alternate.is_none())
            || self.unit_obj_script_registry_generic.is_some() && (self.metadata52.is_none() || self.script_archive.is_none() || self.unit_obj_initial_pose.is_none() || self.unit_obj_equipment.is_none() || self.unit_obj_normal_battle.is_none() || self.unit_obj_alternate_battle.is_none() || self.unit_obj_script_special_alternate.is_none() || self.unit_obj_script_registry_special.is_none())
            || self.unit_obj_property_words.is_some() && (self.metadata52.is_none() || self.metadata14.is_none())
            || self.unit_obj_byte_property_words.is_some() && (self.unit_obj_property_words.is_none() || self.color_mode_palettes.is_none())
            || self.unit_obj_ffff_reader_fields.is_some() && (self.metadata52.is_none() || self.script_archive.is_none())
            || self.slot_job_lookup.is_some() && self.metadata52.is_none()
            || self.slot_job_lookup.is_some() != self.observed_task_parameters.is_some()
            || self.observed_task_parameters.is_some() != self.constructor_obj_animation.is_some()
            || self.constructor_obj_animation.is_some() != self.task_cycle_deltas.is_some()
            || self.task_cycle_deltas.is_some() != self.word_task_obj_animations.is_some()
            || self.word_task_obj_animations.is_some() != self.world_nodes.is_some()
            || self.world_nodes.is_some() != self.world_roads.is_some()
            || self.world_roads.is_some() != self.encounter_headers.is_some()
            || self.encounter_headers.is_some() != self.encounter_unit_templates.is_some()
            || self.encounter_unit_templates.is_some() != self.encounter_coordinates.is_some()
            || self.encounter_coordinates.is_some() != self.metadata28.is_some()
            || self.encounter_coordinates.is_some() != self.records4.is_some()
            || self.records4.is_some() != self.conditional_records8.is_some()
            || self.conditional_records8.is_some() != self.mission_records70.is_some()
            || self.mission_records70.is_some() != self.selection_byte_tables.is_some()
            || self.selection_byte_tables.is_some() != self.event_records12.is_some()
            || self.event_records12.is_some() != self.parameter_records27.is_some()
            || self.parameter_records27.is_some() != self.world_records38.is_some()
            || self.world_records38.is_some() != self.indexed_graphics.is_some()
            || self.indexed_graphics.is_some() != self.ui_raw_transfers.is_some()
            || self.ui_raw_transfers.is_some() != self.dynamic_raw_fragments.is_some()
            || self.dynamic_raw_fragments.is_some() != self.conditional_four_byte_lookup.is_some()
            || self.conditional_four_byte_lookup.is_some() != self.conditional_packed_lookup.is_some()
            || self.conditional_packed_lookup.is_some() != self.conditional_descriptor_banks.is_some()
            || self.conditional_descriptor_banks.is_some() != self.conditional_descriptor_bank_zero.is_some()
            || self.conditional_descriptor_bank_zero.is_some() != self.conditional_property_masks.is_some()
            || self.conditional_property_masks.is_some() != self.compact_obj_animations.is_some()
            || self.compact_obj_animations.is_some() != self.secondary_compact_obj_animations.is_some()
            || self.secondary_compact_obj_animations.is_some() != self.tertiary_compact_obj_animations.is_some()
            || self.tertiary_compact_obj_animations.is_some() != self.quaternary_compact_obj_animations.is_some()
            || self.quaternary_compact_obj_animations.is_some() != self.quinary_compact_obj_animations.is_some()
            || self.quinary_compact_obj_animations.is_some() != self.senary_compact_obj_animations.is_some()
            || self.constructor_obj_animation.is_some() && self.obj_lists.is_none()
            || self.map_raster_palettes.is_some() && self.auxiliary_scripts.is_none()
            || self.auxiliary_scripts.is_some() && self.script_coordinates.is_none()
            || self.manifest.resources.len() != self.resource_count()
        {
            return Err("Extraction resource set is inconsistent".into());
        }
        let (dimensions, file_bytes) = files.dimensions;
        if file_bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("OBJ dimensions file exceeds size limit".into());
        }
        let mut total = file_bytes.len();
        let mut expected = self.manifest.clone();
        expected.resources[0].sha256 = ffta_rom::digest(file_bytes);
        match (&self.background, files.background) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_BG_DESCRIPTORS_FILE_BYTES {
                    return Err("Background descriptor file exceeds size limit".into());
                }
                total = total.saturating_add(bytes.len());
                expected.resources[1].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected background descriptor resource".into()),
        }
        match (&self.palettes, files.palettes) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_BG_BASE_PALETTES_FILE_BYTES {
                    return Err("Base palette file exceeds size limit".into());
                }
                total = total.saturating_add(bytes.len());
                expected.resources[2].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected base palette resource".into()),
        }
        match (&self.alternate_palettes, files.alternate_palettes) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_BG_ALTERNATE_PALETTES_FILE_BYTES {
                    return Err("Alternate palette file exceeds size limit".into());
                }
                total = total.saturating_add(bytes.len());
                expected.resources[3].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected alternate palette resource".into()),
        }
        match (&self.packet_programs, files.packet_programs) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_BG_PACKET_PROGRAMS_FILE_BYTES {
                    return Err("Packet program file exceeds size limit".into());
                }
                total = total.saturating_add(bytes.len());
                expected.resources[4].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected packet program resource".into()),
        }
        match (&self.main_graphics, files.main_graphics) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_BG_MAIN_GRAPHICS_FILE_BYTES {
                    return Err("Main graphics file exceeds size limit".into());
                }
                total = total.saturating_add(bytes.len());
                expected.resources[5].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected main graphics resource".into()),
        }
        match (&self.transfers, files.transfers) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_BG_TRANSFER_PROGRAMS_FILE_BYTES {
                    return Err("Transfer program file exceeds size limit".into());
                }
                total = total.saturating_add(bytes.len());
                expected.resources[6].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected transfer program resource".into()),
        }
        match (&self.short_texts, files.short_texts) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_SHORT_TEXTS_FILE_BYTES {
                    return Err("Short-text file exceeds size limit".into());
                }
                total = total.saturating_add(bytes.len());
                expected.resources[7].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected short-text resource".into()),
        }
        match (&self.text_catalogs, files.text_catalogs) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_TEXT_CATALOGS_FILE_BYTES { return Err("Text catalog file exceeds size limit".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[8].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected text catalog resource".into()),
        }
        match (&self.font_bank, files.font_bank) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_FONT_BANK_FILE_BYTES { return Err("font-bank file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[9].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected font-bank resource".into()),
        }
        match (&self.name_keyboard, files.name_keyboard) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_NAME_KEYBOARD_FILE_BYTES { return Err("name-keyboard file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[10].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected name-keyboard resource".into()),
        }
        match (&self.obj_lists, files.obj_lists) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_OBJ_STATIC_LISTS_FILE_BYTES { return Err("OBJ static-list file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[11].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected OBJ static-list resource".into()),
        }
        match (&self.obj_animations, files.obj_animations) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_OBJ_ANIMATION_BANKS_FILE_BYTES { return Err("OBJ animation file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[12].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected OBJ animation resource".into()),
        }
        match (&self.obj_graphics, files.obj_graphics) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_OBJ_CUSTOM_GRAPHICS_FILE_BYTES { return Err("OBJ custom graphics file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[13].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected OBJ custom graphics resource".into()),
        }
        match (&self.ui_palettes, files.ui_palettes) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UI_RAW_PALETTES_FILE_BYTES { return Err("UI raw palette file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[14].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected UI raw palette resource".into()),
        }
        match (&self.ui_graphics, files.ui_graphics) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UI_CUSTOM_GRAPHICS_FILE_BYTES { return Err("UI custom graphics file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[15].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected UI custom graphics resource".into()),
        }
        match (&self.items, files.items) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_ITEM_RECORDS_FILE_BYTES { return Err("Item file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[16].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected item resource".into()),
        }
        match (&self.item_links, files.item_links) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_ITEM_LINKS_FILE_BYTES { return Err("Item-link file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[17].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected item-link resource".into()),
        }
        match (&self.metadata16, files.metadata16) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_METADATA16_FILE_BYTES { return Err("Metadata16 file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[18].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected metadata16 resource".into()),
        }
        match (&self.metadata52, files.metadata52) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_METADATA52_FILE_BYTES { return Err("Metadata52 file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[19].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected metadata52 resource".into()),
        }
        match (&self.metadata4, files.metadata4) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_METADATA4_FILE_BYTES { return Err("Metadata4 file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[20].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected metadata4 resource".into()),
        }
        match (&self.metadata2, files.metadata2) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_METADATA2_FILE_BYTES { return Err("Metadata2 file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[21].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected metadata2 resource".into()),
        }
        match (&self.default48, files.default48) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_DEFAULT48_FILE_BYTES { return Err("Default48 file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[22].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected default48 resource".into()),
        }
        match (&self.constructor_alternates, files.constructor_alternates) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_CONSTRUCTOR_ALTERNATES_FILE_BYTES { return Err("ConstructorAlternates file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[23].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected constructor_alternates resource".into()),
        }
        match (&self.metadata14, files.metadata14) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_METADATA14_FILE_BYTES { return Err("Metadata14 file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[24].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected metadata14 resource".into()),
        }
        match (&self.metadata12, files.metadata12) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_METADATA12_FILE_BYTES { return Err("Metadata12 file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[25].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected metadata12 resource".into()),
        }
        match (&self.global_pair_flags, files.global_pair_flags) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_GLOBAL_PAIR_FLAGS_FILE_BYTES { return Err("GlobalPairFlags file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[26].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected global_pair_flags resource".into()),
        }
        match (&self.evaluation_parameters, files.evaluation_parameters) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_EVALUATION_PARAMETERS_FILE_BYTES { return Err("EvaluationParameters file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[27].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected evaluation_parameters resource".into()),
        }
        match (&self.progression_rewards, files.progression_rewards) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_PROGRESSION_REWARDS_FILE_BYTES { return Err("ProgressionRewards file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[28].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected progression_rewards resource".into()),
        }
        match (&self.progression_gates, files.progression_gates) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_PROGRESSION_GATES_FILE_BYTES { return Err("ProgressionGates file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[29].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected progression_gates resource".into()),
        }
        match (&self.progression_selection_gates, files.progression_selection_gates) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_PROGRESSION_SELECTION_GATES_FILE_BYTES { return Err("ProgressionSelectionGates file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[30].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected progression_selection_gates resource".into()),
        }
        match (&self.aggregate_flag_keys, files.aggregate_flag_keys) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_AGGREGATE_FLAG_KEYS_FILE_BYTES { return Err("AggregateFlagKeys file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[31].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected aggregate_flag_keys resource".into()),
        }
        match (&self.trade_parameters, files.trade_parameters) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_TRADE_PARAMETERS_FILE_BYTES { return Err("TradeParameters file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[32].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected trade_parameters resource".into()),
        }
        match (&self.item_effect_parameters, files.item_effect_parameters) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_ITEM_EFFECT_PARAMETERS_FILE_BYTES { return Err("ItemEffectParameters file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[33].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected item_effect_parameters resource".into()),
        }
        match (&self.audio_directory, files.audio_directory) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_AUDIO_DIRECTORY_FILE_BYTES { return Err("AudioDirectory file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[34].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected audio_directory resource".into()),
        }
        match (&self.audio_tones, files.audio_tones) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_AUDIO_TONES_FILE_BYTES { return Err("AudioTones file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[35].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected audio_tones resource".into()),
        }
        match (&self.audio_sequences, files.audio_sequences) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_AUDIO_SEQUENCES_FILE_BYTES { return Err("AudioSequences file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[36].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected audio_sequences resource".into()),
        }
        match (&self.audio_driver, files.audio_driver) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_AUDIO_DRIVER_FILE_BYTES { return Err("AudioDriver file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[37].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected audio_driver resource".into()),
        }
        match (&self.palette_effect_dispatch, files.palette_effect_dispatch) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_PALETTE_EFFECT_DISPATCH_FILE_BYTES { return Err("PaletteEffectDispatch file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[38].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected palette_effect_dispatch resource".into()),
        }
        match (&self.palette_programs, files.palette_programs) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_PALETTE_PROGRAMS_FILE_BYTES { return Err("PalettePrograms file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[39].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected palette_programs resource".into()),
        }
        match (&self.blend_programs, files.blend_programs) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_BLEND_PROGRAMS_FILE_BYTES { return Err("BlendPrograms file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[40].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected blend_programs resource".into()),
        }
        match (&self.task_descriptors, files.task_descriptors) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_TASK_DESCRIPTORS_FILE_BYTES { return Err("TaskDescriptors file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[41].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected task_descriptors resource".into()),
        }
        match (&self.effect_event_directories, files.effect_event_directories) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_EFFECT_EVENT_DIRECTORIES_FILE_BYTES { return Err("EffectEventDirectories file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[42].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected effect_event_directories resource".into()),
        }
        match (&self.script_commands, files.script_commands) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_SCRIPT_COMMANDS_FILE_BYTES { return Err("ScriptCommands file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[43].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected script_commands resource".into()),
        }
        match (&self.script_archive, files.script_archive) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_SCRIPT_ARCHIVE_FILE_BYTES { return Err("ScriptArchive file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[44].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected script_archive resource".into()),
        }
        match (&self.script_text, files.script_text) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_SCRIPT_TEXT_FILE_BYTES { return Err("ScriptText file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[45].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected script_text resource".into()),
        }
        match (&self.script_coordinates, files.script_coordinates) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_SCRIPT_COORDINATES_FILE_BYTES { return Err("ScriptCoordinates file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[46].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected script_coordinates resource".into()),
        }
        match (&self.ui_choice_lists, files.ui_choice_lists) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UI_CHOICE_LISTS_FILE_BYTES { return Err("UiChoiceLists file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[50].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected ui_choice_lists resource".into()),
        }
        match (&self.unit_obj_initial_pose, files.unit_obj_initial_pose) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_INITIAL_POSE_FILE_BYTES { return Err("UnitObjInitialPose file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[51].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_initial_pose resource".into()),
        }
        match (&self.color_mode_palettes, files.color_mode_palettes) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_COLOR_MODE_PALETTES_FILE_BYTES { return Err("ColorModePalettes file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[52].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected color_mode_palettes resource".into()),
        }
        match (&self.unit_obj_equipment, files.unit_obj_equipment) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_EQUIPMENT_FILE_BYTES { return Err("UnitObjEquipment file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[53].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_equipment resource".into()),
        }
        match (&self.unit_obj_normal_battle, files.unit_obj_normal_battle) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_NORMAL_BATTLE_FILE_BYTES { return Err("UnitObjNormalBattle file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[54].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_normal_battle resource".into()),
        }
        match (&self.unit_obj_alternate_battle, files.unit_obj_alternate_battle) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_ALTERNATE_BATTLE_FILE_BYTES { return Err("UnitObjAlternateBattle file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[55].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_alternate_battle resource".into()),
        }
        match (&self.unit_obj_script_special_normal, files.unit_obj_script_special_normal) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_SCRIPT_SPECIAL_NORMAL_FILE_BYTES { return Err("UnitObjScriptSpecialNormal file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[56].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_script_special_normal resource".into()),
        }
        match (&self.unit_obj_script_special_alternate, files.unit_obj_script_special_alternate) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_SCRIPT_SPECIAL_ALTERNATE_FILE_BYTES { return Err("UnitObjScriptSpecialAlternate file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[57].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_script_special_alternate resource".into()),
        }
        match (&self.unit_obj_script_override, files.unit_obj_script_override) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_SCRIPT_OVERRIDE_FILE_BYTES { return Err("UnitObjScriptOverride file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[58].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_script_override resource".into()),
        }
        match (&self.unit_obj_script_registry_special, files.unit_obj_script_registry_special) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_SCRIPT_REGISTRY_SPECIAL_FILE_BYTES { return Err("UnitObjScriptRegistrySpecial file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[59].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_script_registry_special resource".into()),
        }
        match (&self.unit_obj_script_registry_generic, files.unit_obj_script_registry_generic) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_SCRIPT_REGISTRY_GENERIC_FILE_BYTES { return Err("UnitObjScriptRegistryGeneric file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[60].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_script_registry_generic resource".into()),
        }
        match (&self.unit_obj_property_words, files.unit_obj_property_words) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_PROPERTY_WORDS_FILE_BYTES { return Err("UnitObjPropertyWords file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[61].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_property_words resource".into()),
        }
        match (&self.unit_obj_byte_property_words, files.unit_obj_byte_property_words) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_BYTE_PROPERTY_WORDS_FILE_BYTES { return Err("UnitObjBytePropertyWords file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[62].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_byte_property_words resource".into()),
        }
        match (&self.unit_obj_ffff_reader_fields, files.unit_obj_ffff_reader_fields) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_UNIT_OBJ_FFFF_READER_FIELDS_FILE_BYTES { return Err("UnitObjFfffReaderFields file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[63].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected unit_obj_ffff_reader_fields resource".into()),
        }
        match (&self.slot_job_lookup, files.slot_job_lookup) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_SLOT_JOB_LOOKUP_FILE_BYTES { return Err("SlotJobLookup file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[64].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected slot_job_lookup resource".into()),
        }
        match (&self.observed_task_parameters, files.observed_task_parameters) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_OBSERVED_TASK_PARAMETERS_FILE_BYTES { return Err("ObservedTaskParameters file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[65].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected observed_task_parameters resource".into()),
        }
        match (&self.constructor_obj_animation, files.constructor_obj_animation) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_CONSTRUCTOR_OBJ_ANIMATION_FILE_BYTES { return Err("ConstructorObjAnimation file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[66].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected constructor_obj_animation resource".into()),
        }
        match (&self.task_cycle_deltas, files.task_cycle_deltas) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_TASK_CYCLE_DELTAS_FILE_BYTES { return Err("TaskCycleDeltas file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[67].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected task_cycle_deltas resource".into()),
        }
        match (&self.word_task_obj_animations, files.word_task_obj_animations) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_WORD_TASK_OBJ_ANIMATIONS_FILE_BYTES { return Err("WordTaskObjAnimations file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[68].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected word_task_obj_animations resource".into()),
        }
        match (&self.world_nodes, files.world_nodes) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_WORLD_NODES_FILE_BYTES { return Err("WorldNodes file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[69].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected world_nodes resource".into()),
        }
        match (&self.world_roads, files.world_roads) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_WORLD_ROADS_FILE_BYTES { return Err("WorldRoads file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[70].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected world_roads resource".into()),
        }
        match (&self.encounter_headers, files.encounter_headers) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_ENCOUNTER_HEADERS_FILE_BYTES { return Err("EncounterHeaders file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[71].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected encounter_headers resource".into()),
        }
        match (&self.encounter_unit_templates, files.encounter_unit_templates) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_ENCOUNTER_UNIT_TEMPLATES_FILE_BYTES { return Err("EncounterUnitTemplates file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[72].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected encounter_unit_templates resource".into()),
        }
        match (&self.encounter_coordinates, files.encounter_coordinates) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_ENCOUNTER_COORDINATES_FILE_BYTES {return Err("Encounter coordinates file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[73].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected encounter coordinates".into()),
        }
        match (&self.metadata28, files.metadata28) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_METADATA28_FILE_BYTES {return Err("Metadata28 file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[74].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected metadata28".into()),
        }
        match (&self.records4, files.records4) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_RECORDS4_FILE_BYTES {return Err("Records4 file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[75].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected records4".into()),
        }
        match (&self.conditional_records8, files.conditional_records8) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_CONDITIONAL_RECORDS8_FILE_BYTES {return Err("ConditionalRecords8 file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[76].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected conditional_records8".into()),
        }
        match (&self.mission_records70, files.mission_records70) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_MISSION_RECORDS70_FILE_BYTES {return Err("MissionRecords70 file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[77].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected mission_records70".into()),
        }
        match (&self.selection_byte_tables, files.selection_byte_tables) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_SELECTION_BYTE_TABLES_FILE_BYTES {return Err("SelectionByteTables file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[78].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected selection_byte_tables".into()),
        }
        match (&self.event_records12, files.event_records12) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_EVENT_RECORDS12_FILE_BYTES {return Err("EventRecords12 file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[79].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected event_records12".into()),
        }
        match (&self.parameter_records27, files.parameter_records27) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_PARAMETER_RECORDS27_FILE_BYTES {return Err("ParameterRecords27 file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[80].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected parameter_records27".into()),
        }
        match (&self.world_records38, files.world_records38) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_WORLD_RECORDS38_FILE_BYTES {return Err("WorldRecords38 file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[81].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected world_records38".into()),
        }
        match (&self.indexed_graphics, files.indexed_graphics) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_INDEXED_GRAPHICS_FILE_BYTES {return Err("IndexedGraphics file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[82].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected indexed_graphics".into()),
        }
        match (&self.ui_raw_transfers, files.ui_raw_transfers) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_UI_RAW_TRANSFERS_FILE_BYTES {return Err("RawTransfers file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[83].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected ui_raw_transfers".into()),
        }
        match (&self.dynamic_raw_fragments, files.dynamic_raw_fragments) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_DYNAMIC_RAW_FRAGMENTS_FILE_BYTES {return Err("Dynamic fragments file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[84].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected dynamic_raw_fragments".into()),
        }
        match (&self.conditional_four_byte_lookup, files.conditional_four_byte_lookup) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_CONDITIONAL_FOUR_BYTE_LOOKUP_FILE_BYTES {return Err("Conditional lookup file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[85].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected conditional_four_byte_lookup".into()),
        }
        match (&self.conditional_packed_lookup, files.conditional_packed_lookup) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_CONDITIONAL_PACKED_LOOKUP_FILE_BYTES {return Err("Conditional packed lookup file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[86].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected conditional_packed_lookup".into()),
        }
        match (&self.conditional_descriptor_banks, files.conditional_descriptor_banks) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_CONDITIONAL_DESCRIPTOR_BANKS_FILE_BYTES {return Err("Conditional descriptor banks file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[87].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected conditional_descriptor_banks".into()),
        }
        match (&self.conditional_descriptor_bank_zero, files.conditional_descriptor_bank_zero) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_CONDITIONAL_DESCRIPTOR_BANK_ZERO_FILE_BYTES {return Err("Conditional descriptor bank zero file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[88].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected conditional_descriptor_bank_zero".into()),
        }
        match (&self.conditional_property_masks, files.conditional_property_masks) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_CONDITIONAL_PROPERTY_MASKS_FILE_BYTES {return Err("Conditional property masks file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[89].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected conditional_property_masks".into()),
        }
        match (&self.compact_obj_animations, files.compact_obj_animations) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_COMPACT_OBJ_ANIMATIONS_FILE_BYTES {return Err("Compact OBJ animations file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[90].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected compact_obj_animations".into()),
        }
        match (&self.secondary_compact_obj_animations, files.secondary_compact_obj_animations) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_SECONDARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES {return Err("Secondary compact OBJ animations file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[91].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected secondary_compact_obj_animations".into()),
        }
        match (&self.tertiary_compact_obj_animations, files.tertiary_compact_obj_animations) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_TERTIARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES {return Err("Tertiary compact OBJ animations file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[92].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected tertiary_compact_obj_animations".into()),
        }
        match (&self.quaternary_compact_obj_animations, files.quaternary_compact_obj_animations) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_QUATERNARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES {return Err("Quaternary compact OBJ animations file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[93].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected quaternary_compact_obj_animations".into()),
        }
        match (&self.quinary_compact_obj_animations, files.quinary_compact_obj_animations) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_QUINARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES {return Err("Quinary compact OBJ animations file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[94].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected quinary_compact_obj_animations".into()),
        }
        match (&self.senary_compact_obj_animations, files.senary_compact_obj_animations) {
            (Some(_),Some((_,bytes)))=>{if bytes.len() as u64>MAX_SENARY_COMPACT_OBJ_ANIMATIONS_FILE_BYTES {return Err("Senary compact OBJ animations file exceeds input guard".into());}total=total.saturating_add(bytes.len());expected.resources[95].sha256=ffta_rom::digest(bytes);}
            (None,None)=>{},_=>return Err("Missing or unexpected senary_compact_obj_animations".into()),
        }
        match (&self.map_raster_palettes, files.map_raster_palettes) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_MAP_RASTER_PALETTES_FILE_BYTES { return Err("MapRasterPalettes file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[49].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected map_raster_palettes resource".into()),
        }
        match (&self.auxiliary_scripts, files.auxiliary_commands) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_AUXILIARY_COMMANDS_FILE_BYTES { return Err("AuxiliaryCommands file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[47].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected auxiliary_commands resource".into()),
        }
        match (&self.auxiliary_scripts, files.auxiliary_archive) {
            (Some(_), Some((_, bytes))) => {
                if bytes.len() as u64 > MAX_AUXILIARY_ARCHIVE_FILE_BYTES { return Err("AuxiliaryArchive file exceeds input guard".into()); }
                total = total.saturating_add(bytes.len());
                expected.resources[48].sha256 = ffta_rom::digest(bytes);
            }
            (None, None) => {}
            _ => return Err("Missing or unexpected auxiliary_archive resource".into()),
        }

        if total > MAX_DATASET_BYTES {
            return Err("Resource files exceed total size limit".into());
        }
        if files.manifest != &expected {
            return Err(
                "Resource manifest does not match the source/profile/schema or file hash".into(),
            );
        }
        dimensions.validate()?;
        if dimensions != &self.dimensions
            || ffta_rom::digest(&dimensions.source_bytes())
                != self.manifest.resources[0].sources[0].sha256
        {
            return Err("Decoded OBJ dimensions do not reproduce the selected ROM source".into());
        }
        if let (Some(expected), Some((decoded, _))) = (&self.background, files.background) {
            let source = &self.manifest.resources[1].sources[0];
            decoded.validate(source.offset, self.manifest.source_rom_size as usize)?;
            if decoded != expected || ffta_rom::digest(&decoded.source_bytes()?) != source.sha256 {
                return Err(
                    "Decoded background descriptors do not reproduce the selected ROM source"
                        .into(),
                );
            }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.palettes, files.palettes) {
            decoded.validate()?;
            if decoded != &expected.decoded {
                return Err(
                    "Decoded base palettes do not reproduce the selected ROM source".into(),
                );
            }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.alternate_palettes, files.alternate_palettes) {
            decoded.validate()?;
            if decoded != &expected.decoded {
                return Err("Decoded alternate palettes do not reproduce the selected ROM source".into());
            }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.packet_programs, files.packet_programs) {
            decoded.validate()?;
            if decoded != &expected.decoded {
                return Err("Decoded packet programs do not reproduce the selected ROM source".into());
            }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.main_graphics, files.main_graphics) {
            decoded.validate()?;
            if decoded != &expected.decoded {
                return Err("Decoded main graphics do not reproduce the selected ROM source".into());
            }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.transfers, files.transfers) {
            decoded.validate()?;
            if decoded != &expected.decoded {
                return Err("Decoded transfer programs do not reproduce the selected ROM source".into());
            }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.short_texts, files.short_texts) {
            decoded.validate()?;
            if decoded != &expected.decoded {
                return Err("Decoded short texts do not reproduce the selected ROM source".into());
            }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.text_catalogs, files.text_catalogs) {
            decoded.validate()?;
            if decoded != &expected.decoded { return Err("Decoded text catalogs do not reproduce selected ROM source".into()); }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.font_bank, files.font_bank) {
            decoded.validate()?;
            if decoded != &expected.decoded { return Err("Decoded font-bank differs from authenticated source".into()); }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.name_keyboard, files.name_keyboard) {
            decoded.validate()?;
            if decoded != &expected.decoded { return Err("Decoded name-keyboard differs from authenticated source".into()); }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.obj_lists, files.obj_lists) {
            decoded.validate()?;
            if decoded != &expected.decoded { return Err("Decoded OBJ static lists differ from authenticated source".into()); }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.obj_animations, files.obj_animations) {
            decoded.validate()?;
            if decoded != &expected.decoded { return Err("Decoded OBJ animation banks differ from authenticated source".into()); }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.obj_graphics, files.obj_graphics) {
            decoded.validate()?;
            if decoded != &expected.decoded { return Err("Decoded OBJ custom graphics differ from authenticated source".into()); }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.ui_palettes, files.ui_palettes) {
            decoded.validate()?;
            if decoded != &expected.decoded { return Err("Decoded UI raw palettes differ from authenticated source".into()); }
        }
        if let (Some(expected), Some((decoded, _))) = (&self.ui_graphics, files.ui_graphics) {
            decoded.validate()?;
            decoded.validate_shared_obj(files.obj_graphics.ok_or("Missing shared OBJ source owner")?.0)?;
            if decoded != &expected.decoded { return Err("Decoded UI custom banks differ from authenticated source".into()); }
        }
        if let (Some(expected),Some((decoded,_))) = (&self.items, files.items) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded items differ from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.item_links, files.item_links) {
            decoded.validate_items(files.items.ok_or("Missing item-link item owner")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded item links differ from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.metadata16, files.metadata16) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded metadata16 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.metadata52, files.metadata52) {
            decoded.validate_links(files.item_links.ok_or("Missing metadata52 link owner")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded metadata52 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.metadata4, files.metadata4) {
            decoded.validate_metadata52(files.metadata52.ok_or("Missing metadata4 owner")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded metadata4 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.metadata2, files.metadata2) {
            decoded.validate_metadata52(files.metadata52.ok_or("Missing metadata2 owner")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded metadata2 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.default48, files.default48) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded default48 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.constructor_alternates, files.constructor_alternates) {
            decoded.validate_items(files.items.ok_or("Missing constructor-alternate item owner")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded constructor_alternates differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.metadata14, files.metadata14) {
            decoded.validate_metadata52(files.metadata52.ok_or("Missing metadata14 job owner")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded metadata14 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.metadata12, files.metadata12) {
            if decoded != &expected.decoded {return Err("Decoded metadata12 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.global_pair_flags, files.global_pair_flags) {
            if decoded != &expected.decoded {return Err("Decoded global_pair_flags differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.evaluation_parameters, files.evaluation_parameters) {
            if decoded != &expected.decoded {return Err("Decoded evaluation_parameters differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.progression_rewards, files.progression_rewards) {
            if decoded != &expected.decoded {return Err("Decoded progression_rewards differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.progression_gates, files.progression_gates) {
            if decoded != &expected.decoded {return Err("Decoded progression_gates differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.progression_selection_gates, files.progression_selection_gates) {
            if decoded != &expected.decoded {return Err("Decoded progression_selection_gates differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.aggregate_flag_keys, files.aggregate_flag_keys) {
            if decoded != &expected.decoded {return Err("Decoded aggregate_flag_keys differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.trade_parameters, files.trade_parameters) {
            if decoded != &expected.decoded {return Err("Decoded trade_parameters differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.item_effect_parameters, files.item_effect_parameters) {
            decoded.validate_items(files.items.ok_or("Missing item-effect item owner")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded item_effect_parameters differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.audio_directory, files.audio_directory) {
            if decoded != &expected.decoded {return Err("Decoded audio_directory differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.audio_tones, files.audio_tones) {
            decoded.validate_directory(files.audio_directory.ok_or("Missing actual audio-directory dependency")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded audio_tones differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.audio_sequences, files.audio_sequences) {
            decoded.validate_owners(files.audio_directory.ok_or("Missing actual audio-directory dependency")?.0,files.audio_tones.ok_or("Missing actual audio-tone dependency")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded audio_sequences differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.audio_driver, files.audio_driver) {
            decoded.validate_sequence_owner(files.audio_sequences.ok_or("Missing actual audio-sequence dependency")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded audio_driver differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.palette_effect_dispatch, files.palette_effect_dispatch) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded palette_effect_dispatch differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.palette_programs, files.palette_programs) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded palette_programs differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.blend_programs, files.blend_programs) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded blend_programs differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.task_descriptors, files.task_descriptors) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded task_descriptors differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.effect_event_directories, files.effect_event_directories) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded effect_event_directories differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.script_commands, files.script_commands) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded script_commands differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.script_archive, files.script_archive) {
            let commands=files.script_commands.ok_or("Missing script archive command dependency")?.0;
            let effects=files.effect_event_directories.ok_or("Missing script archive effect dependency")?.0;
            decoded.validate_dependencies(commands,effects)?;
            if decoded != &expected.decoded {return Err("Decoded script_archive differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.script_text, files.script_text) {
            let keys=files.script_archive.ok_or("Missing script text key dependency")?.0;
            let font=files.font_bank.ok_or("Missing script text font dependency")?.0;
            decoded.validate_dependencies(keys,font)?;
            if decoded != &expected.decoded {return Err("Decoded script_text differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.script_coordinates, files.script_coordinates) {
            let archive=files.script_archive.ok_or("Missing coordinate archive dependency")?.0;
            let commands=files.script_commands.ok_or("Missing coordinate command dependency")?.0;
            decoded.validate_dependencies(archive,commands)?;
            if decoded != &expected.decoded {return Err("Decoded script_coordinates differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.ui_choice_lists, files.ui_choice_lists) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded ui_choice_lists differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_initial_pose, files.unit_obj_initial_pose) {
            let jobs=files.metadata52.ok_or("Missing unit OBJ job dependency")?.0;
            let special=files.metadata14.ok_or("Missing unit OBJ special dependency")?.0;
            decoded.validate_producers(jobs,special)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_initial_pose differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.color_mode_palettes, files.color_mode_palettes) {
            let special=files.metadata14.ok_or("Missing Color-Mode palette metadata14 dependency")?.0;
            runtime::color_mode_palettes::validate_dependencies(decoded,special)?;
            if decoded != &expected.decoded {return Err("Decoded color_mode_palettes differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_equipment, files.unit_obj_equipment) {
            let items=files.items.ok_or("Missing equipment OBJ item dependency")?.0;
            decoded.validate_producers(items)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_equipment differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_normal_battle, files.unit_obj_normal_battle) {
            let jobs=files.metadata52.ok_or("Missing normal battle job dependency")?.0;
            let special=files.metadata14.ok_or("Missing normal battle special dependency")?.0;
            let initial=files.unit_obj_initial_pose.ok_or("Missing normal battle initial dependency")?.0;
            let equipment=files.unit_obj_equipment.ok_or("Missing normal battle equipment dependency")?.0;
            decoded.validate_producers(jobs,special)?;
            decoded.owned_spans(initial,equipment)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_normal_battle differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_alternate_battle, files.unit_obj_alternate_battle) {
            let jobs=files.metadata52.ok_or("Missing alternate battle job dependency")?.0;
            let special=files.metadata14.ok_or("Missing alternate battle special dependency")?.0;
            let initial=files.unit_obj_initial_pose.ok_or("Missing alternate battle initial dependency")?.0;
            let equipment=files.unit_obj_equipment.ok_or("Missing alternate battle equipment dependency")?.0;
            let normal=files.unit_obj_normal_battle.ok_or("Missing alternate battle normal dependency")?.0;
            decoded.validate_producers(jobs,special)?;
            decoded.owned_spans(initial,equipment,normal)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_alternate_battle differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_script_special_normal, files.unit_obj_script_special_normal) {
            let special=files.metadata14.ok_or("Missing script-special normal special dependency")?.0;
            let initial=files.unit_obj_initial_pose.ok_or("Missing script-special normal initial dependency")?.0;
            let normal=files.unit_obj_normal_battle.ok_or("Missing script-special normal normal dependency")?.0;
            let scripts=files.script_archive.ok_or("Missing script-special normal archive dependency")?.0;
            decoded.validate_producers(scripts,special)?;
            decoded.owned_spans(initial,normal)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_script_special_normal differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_script_special_alternate, files.unit_obj_script_special_alternate) {
            let special=files.metadata14.ok_or("Missing script-special alternate special dependency")?.0;
            let initial=files.unit_obj_initial_pose.ok_or("Missing script-special alternate initial dependency")?.0;
            let normal=files.unit_obj_normal_battle.ok_or("Missing script-special alternate normal dependency")?.0;
            let scripts=files.script_archive.ok_or("Missing script-special alternate archive dependency")?.0;
            decoded.validate_producers(scripts,special)?;
            let alternate=files.unit_obj_alternate_battle.ok_or("Missing script-special alternate alternate dependency")?.0;
            decoded.owned_spans(initial,normal,alternate)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_script_special_alternate differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_script_override, files.unit_obj_script_override) {
            let normal=files.unit_obj_normal_battle.ok_or("Missing script override normal dependency")?.0;
            let scripts=files.script_archive.ok_or("Missing script override archive dependency")?.0;
            decoded.validate_producers(scripts)?;
            let alternate=files.unit_obj_alternate_battle.ok_or("Missing script override alternate dependency")?.0;
            decoded.owned_spans(normal,alternate)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_script_override differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_script_registry_special, files.unit_obj_script_registry_special) {
            let special=files.metadata14.ok_or("Missing registry-special script metadata14 dependency")?.0;
            let scripts=files.script_archive.ok_or("Missing registry-special script script_archive dependency")?.0;
            let initial=files.unit_obj_initial_pose.ok_or("Missing registry-special script unit_obj_initial_pose dependency")?.0;
            let equipment=files.unit_obj_equipment.ok_or("Missing registry-special script unit_obj_equipment dependency")?.0;
            let normal=files.unit_obj_normal_battle.ok_or("Missing registry-special script unit_obj_normal_battle dependency")?.0;
            let alternate=files.unit_obj_alternate_battle.ok_or("Missing registry-special script unit_obj_alternate_battle dependency")?.0;
            let low_normal=files.unit_obj_script_special_normal.ok_or("Missing registry-special script unit_obj_script_special_normal dependency")?.0;
            let low_alternate=files.unit_obj_script_special_alternate.ok_or("Missing registry-special script unit_obj_script_special_alternate dependency")?.0;
            decoded.validate_producers(scripts,special)?;
            decoded.owned_spans(initial,equipment,normal,alternate,low_normal,low_alternate)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_script_registry_special differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_script_registry_generic, files.unit_obj_script_registry_generic) {
            let jobs=files.metadata52.ok_or("Missing registry-generic script metadata52 dependency")?.0;
            let scripts=files.script_archive.ok_or("Missing registry-generic script script_archive dependency")?.0;
            let initial=files.unit_obj_initial_pose.ok_or("Missing registry-generic script unit_obj_initial_pose dependency")?.0;
            let equipment=files.unit_obj_equipment.ok_or("Missing registry-generic script unit_obj_equipment dependency")?.0;
            let normal=files.unit_obj_normal_battle.ok_or("Missing registry-generic script unit_obj_normal_battle dependency")?.0;
            let alternate=files.unit_obj_alternate_battle.ok_or("Missing registry-generic script unit_obj_alternate_battle dependency")?.0;
            let low_alternate=files.unit_obj_script_special_alternate.ok_or("Missing registry-generic script unit_obj_script_special_alternate dependency")?.0;
            let known_special=files.unit_obj_script_registry_special.ok_or("Missing registry-generic script unit_obj_script_registry_special dependency")?.0;
            decoded.validate_producers(scripts,jobs)?;
            decoded.owned_spans(initial,equipment,normal,alternate,low_alternate,known_special)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_script_registry_generic differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_property_words, files.unit_obj_property_words) {
            let jobs=files.metadata52.ok_or("Missing actor propertyword metadata52 dependency")?.0;
            let special=files.metadata14.ok_or("Missing actor propertyword metadata14 dependency")?.0;
            decoded.validate_producers(jobs,special)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_property_words differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_byte_property_words, files.unit_obj_byte_property_words) {
            let properties=files.unit_obj_property_words.ok_or("Missing UI byte propertyword actor-property dependency")?.0;
            let palettes=files.color_mode_palettes.ok_or("Missing UI byte propertyword palette dependency")?.0;
            decoded.validate_shared(properties,palettes)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_byte_property_words differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.unit_obj_ffff_reader_fields, files.unit_obj_ffff_reader_fields) {
            let jobs=files.metadata52.ok_or("Missing FFFF reader metadata52 dependency")?.0;
            let scripts=files.script_archive.ok_or("Missing FFFF reader script archive dependency")?.0;
            decoded.validate_dependencies(scripts,jobs)?;
            if decoded != &expected.decoded {return Err("Decoded unit_obj_ffff_reader_fields differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.slot_job_lookup, files.slot_job_lookup) {
            let jobs=files.metadata52.ok_or("Missing slot-job metadata52 dependency")?.0;
            decoded.validate_dependencies(jobs)?;
            if decoded != &expected.decoded {return Err("Decoded slot_job_lookup differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.observed_task_parameters, files.observed_task_parameters) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded observed_task_parameters differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.constructor_obj_animation, files.constructor_obj_animation) {
            decoded.validate_lists(files.obj_lists.ok_or("Missing constructor animation OBJ-list dependency")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded constructor_obj_animation differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.task_cycle_deltas, files.task_cycle_deltas) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded task_cycle_deltas differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.word_task_obj_animations, files.word_task_obj_animations) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded word_task_obj_animations differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.world_nodes, files.world_nodes) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded world_nodes differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.world_roads, files.world_roads) {
            decoded.validate()?;
            if decoded != &expected.decoded {return Err("Decoded world_roads differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.encounter_headers, files.encounter_headers) {
            decoded.validate_constructor_alternates(files.constructor_alternates.ok_or("Missing encounter constructor dependency")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded encounter_headers differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.encounter_unit_templates, files.encounter_unit_templates) {
            decoded.validate_constructor_alternates(files.constructor_alternates.ok_or("Missing encounter constructor dependency")?.0)?;
            decoded.validate_headers(files.encounter_headers.ok_or("Missing encounter header dependency")?.0)?;
            decoded.validate_references(files.items.ok_or("Missing encounter items dependency")?.0, files.metadata14.ok_or("Missing encounter descriptor dependency")?.0, files.metadata52.ok_or("Missing encounter job dependency")?.0)?;
            if decoded != &expected.decoded {return Err("Decoded encounter_unit_templates differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.encounter_coordinates,files.encounter_coordinates) {
            decoded.validate_headers(files.encounter_headers.ok_or("Missing coordinate header dependency")?.0)?;
            if decoded!=&expected.decoded {return Err("Decoded coordinates differ from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.metadata28,files.metadata28) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded metadata28 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.records4,files.records4) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded records4 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.conditional_records8,files.conditional_records8) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded conditional_records8 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.mission_records70,files.mission_records70) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded mission_records70 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.selection_byte_tables,files.selection_byte_tables) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded selection_byte_tables differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.event_records12,files.event_records12) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded event_records12 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.parameter_records27,files.parameter_records27) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded parameter_records27 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.world_records38,files.world_records38) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded world_records38 differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.indexed_graphics,files.indexed_graphics) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded indexed_graphics differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.ui_raw_transfers,files.ui_raw_transfers) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded ui_raw_transfers differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.dynamic_raw_fragments,files.dynamic_raw_fragments) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded dynamic_raw_fragments differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.conditional_four_byte_lookup,files.conditional_four_byte_lookup) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded conditional_four_byte_lookup differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.conditional_packed_lookup,files.conditional_packed_lookup) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded conditional_packed_lookup differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.conditional_descriptor_banks,files.conditional_descriptor_banks) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded conditional_descriptor_banks differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.conditional_descriptor_bank_zero,files.conditional_descriptor_bank_zero) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded conditional_descriptor_bank_zero differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.conditional_property_masks,files.conditional_property_masks) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded conditional_property_masks differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.compact_obj_animations,files.compact_obj_animations) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded compact_obj_animations differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.secondary_compact_obj_animations,files.secondary_compact_obj_animations) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded secondary_compact_obj_animations differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.tertiary_compact_obj_animations,files.tertiary_compact_obj_animations) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded tertiary_compact_obj_animations differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.quaternary_compact_obj_animations,files.quaternary_compact_obj_animations) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded quaternary_compact_obj_animations differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.quinary_compact_obj_animations,files.quinary_compact_obj_animations) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded quinary_compact_obj_animations differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_)))=(&self.senary_compact_obj_animations,files.senary_compact_obj_animations) {
            decoded.validate()?;
            if decoded!=&expected.decoded {return Err("Decoded senary_compact_obj_animations differs from authenticated source".into());}
        }
        if let (Some(expected),Some((decoded,_))) = (&self.map_raster_palettes, files.map_raster_palettes) {
            let descriptors=files.background.ok_or("Missing raster palette descriptor dependency")?.0;
            decoded.validate_dependencies(descriptors)?;
            if decoded != &expected.decoded {return Err("Decoded map_raster_palettes differs from authenticated source".into());}
        }
        if let Some(expected) = &self.auxiliary_scripts {
            let commands=files.auxiliary_commands.ok_or("Missing auxiliary command dependency")?.0;
            let archive=files.auxiliary_archive.ok_or("Missing auxiliary archive resource")?.0;
            commands.validate()?;
            archive.validate_dependencies(commands)?;
            if commands != &expected.commands || archive != &expected.decoded {return Err("Decoded auxiliary scripts differ from authenticated source".into());}
        }
        Ok(())
    }

    fn resource_count(&self) -> usize {
        1 + usize::from(self.background.is_some()) + usize::from(self.palettes.is_some())
            + usize::from(self.alternate_palettes.is_some())
            + usize::from(self.packet_programs.is_some())
            + usize::from(self.main_graphics.is_some())
            + usize::from(self.transfers.is_some())
            + usize::from(self.short_texts.is_some())
            + usize::from(self.text_catalogs.is_some())
            + usize::from(self.name_keyboard.is_some())
            + usize::from(self.font_bank.is_some())
            + usize::from(self.obj_lists.is_some())
            + usize::from(self.obj_animations.is_some())
            + usize::from(self.obj_graphics.is_some())
            + usize::from(self.ui_palettes.is_some())
            + usize::from(self.ui_graphics.is_some())
            + usize::from(self.items.is_some())
            + usize::from(self.item_links.is_some())
            + usize::from(self.metadata16.is_some())
            + usize::from(self.metadata52.is_some())
            + usize::from(self.metadata4.is_some())
            + usize::from(self.metadata2.is_some())
            + usize::from(self.default48.is_some())
            + usize::from(self.constructor_alternates.is_some())
            + usize::from(self.metadata14.is_some())
            + usize::from(self.metadata12.is_some())
            + usize::from(self.global_pair_flags.is_some())
            + usize::from(self.evaluation_parameters.is_some())
            + usize::from(self.progression_rewards.is_some())
            + usize::from(self.progression_gates.is_some())
            + usize::from(self.progression_selection_gates.is_some())
            + usize::from(self.aggregate_flag_keys.is_some())
            + usize::from(self.trade_parameters.is_some())
            + usize::from(self.item_effect_parameters.is_some())
            + usize::from(self.audio_directory.is_some())
            + usize::from(self.audio_tones.is_some())
            + usize::from(self.audio_sequences.is_some())
            + usize::from(self.audio_driver.is_some())
            + usize::from(self.palette_effect_dispatch.is_some())
            + usize::from(self.palette_programs.is_some())
            + usize::from(self.blend_programs.is_some())
            + usize::from(self.task_descriptors.is_some())
            + usize::from(self.effect_event_directories.is_some())
            + usize::from(self.script_commands.is_some())
            + usize::from(self.script_archive.is_some())
            + usize::from(self.script_text.is_some())
            + usize::from(self.script_coordinates.is_some())
            + usize::from(self.ui_choice_lists.is_some())
            + usize::from(self.unit_obj_initial_pose.is_some())
            + usize::from(self.color_mode_palettes.is_some())
            + usize::from(self.unit_obj_equipment.is_some())
            + usize::from(self.unit_obj_normal_battle.is_some())
            + usize::from(self.unit_obj_alternate_battle.is_some())
            + usize::from(self.unit_obj_script_special_normal.is_some())
            + usize::from(self.unit_obj_script_special_alternate.is_some())
            + usize::from(self.unit_obj_script_override.is_some())
            + usize::from(self.unit_obj_script_registry_special.is_some())
            + usize::from(self.unit_obj_script_registry_generic.is_some())
            + usize::from(self.unit_obj_property_words.is_some())
            + usize::from(self.unit_obj_byte_property_words.is_some())
            + usize::from(self.unit_obj_ffff_reader_fields.is_some())
            + usize::from(self.slot_job_lookup.is_some())
            + usize::from(self.observed_task_parameters.is_some())
            + usize::from(self.constructor_obj_animation.is_some())
            + usize::from(self.task_cycle_deltas.is_some())
            + usize::from(self.word_task_obj_animations.is_some())
            + usize::from(self.world_nodes.is_some())
            + usize::from(self.world_roads.is_some())
            + usize::from(self.encounter_headers.is_some())
            + usize::from(self.encounter_unit_templates.is_some())
            + usize::from(self.encounter_coordinates.is_some())
            + usize::from(self.metadata28.is_some())
            + usize::from(self.records4.is_some())
            + usize::from(self.conditional_records8.is_some())
            + usize::from(self.mission_records70.is_some())
            + usize::from(self.selection_byte_tables.is_some())
            + usize::from(self.event_records12.is_some())
            + usize::from(self.parameter_records27.is_some())
            + usize::from(self.world_records38.is_some())
            + usize::from(self.indexed_graphics.is_some())
            + usize::from(self.ui_raw_transfers.is_some())
            + usize::from(self.dynamic_raw_fragments.is_some())
            + usize::from(self.conditional_four_byte_lookup.is_some())
            + usize::from(self.conditional_packed_lookup.is_some())
            + usize::from(self.conditional_descriptor_banks.is_some())
            + usize::from(self.conditional_descriptor_bank_zero.is_some())
            + usize::from(self.conditional_property_masks.is_some())
            + usize::from(self.compact_obj_animations.is_some())
            + usize::from(self.secondary_compact_obj_animations.is_some())
            + usize::from(self.tertiary_compact_obj_animations.is_some())
            + usize::from(self.quaternary_compact_obj_animations.is_some())
            + usize::from(self.quinary_compact_obj_animations.is_some())
            + usize::from(self.senary_compact_obj_animations.is_some())
            + usize::from(self.map_raster_palettes.is_some())
            + 2 * usize::from(self.auxiliary_scripts.is_some())
    }

    pub fn version(&self) -> Version {
        if self.manifest.profile == "usa" {
            Version::Usa
        } else {
            Version::Europe
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> [u8; 48] {
        let mut bytes = [0; 48];
        for i in 0..12 {
            bytes[i * 4..i * 4 + 2].copy_from_slice(&(8u16 << (i % 4)).to_le_bytes());
            bytes[i * 4 + 2..i * 4 + 4].copy_from_slice(&(8u16 << ((i + 1) % 4)).to_le_bytes());
        }
        bytes
    }

    #[test]
    fn dimensions_roundtrip_and_invalid_inputs() {
        let bytes = fixture();
        let decoded = ObjDimensions::decode(&bytes).unwrap();
        assert_eq!(decoded.source_bytes(), bytes);
        assert_eq!(decoded.get(2, 3), Some(&decoded.entries[11]));
        assert!(decoded.get(3, 0).is_none());
        assert!(decoded.get(0, 4).is_none());
        assert!(decoded.get(255, 255).is_none());
        for len in 0..48 {
            assert!(ObjDimensions::decode(&bytes[..len]).is_err());
        }
        assert!(ObjDimensions::decode(&[0; 49]).is_err());
        let mut invalid = bytes;
        invalid[0..2].copy_from_slice(&256u16.to_le_bytes());
        assert!(ObjDimensions::decode(&invalid).is_err());
        assert!(ObjDimensions::decode(&[0; 48]).is_err());
    }

    #[test]
    fn independent_aliases_bounds_and_base_authentication() {
        let bytes = fixture();
        let hash = ffta_rom::digest(&bytes);
        let mut rom = bytes.repeat(2);
        let ranges = [(0, "direct"), (48, "iwram-copy")];
        let extraction =
            Extraction::from_ranges(&rom, "synthetic".into(), "test", &ranges, &hash).unwrap();
        let mut manifest = extraction.manifest.clone();
        manifest.resources[0].sha256 = ffta_rom::digest(b"file");
        extraction
            .validate_files(BaseFiles {
                manifest: &manifest,
                dimensions: (&extraction.dimensions, b"file"),
                background: None,
                palettes: None,
                alternate_palettes: None,
                packet_programs: None,
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
            auxiliary_commands: None,
            auxiliary_archive: None,
            font_bank: None,
            })
            .unwrap();
        let mut edited = extraction.dimensions.clone();
        edited.entries[0].width = 64;
        assert!(extraction
            .validate_files(BaseFiles {
                manifest: &manifest,
                dimensions: (&edited, b"file"),
                background: None,
                palettes: None,
                alternate_palettes: None,
                packet_programs: None,
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
            auxiliary_commands: None,
            auxiliary_archive: None,
            font_bank: None,
            })
            .is_err());
        for mutate in [0, 1, 2, 3, 4, 5, 6, 7] {
            let mut bad = manifest.clone();
            match mutate {
                0 => bad.schema += 1,
                1 => bad.decoder_revision += 1,
                2 => bad.extractor_revision += 1,
                3 => bad.source_rom_sha256 = "edited".into(),
                4 => bad.resources[0].path = "../escape".into(),
                5 => bad.resources.push(bad.resources[0].clone()),
                6 => bad.resources[0].sources[0].offset += 1,
                _ => bad.coverage.push("unverified".into()),
            }
            assert!(extraction
                .validate_files(BaseFiles {
                    manifest: &bad,
                    dimensions: (&extraction.dimensions, b"file"),
                    background: None,
                    palettes: None,
                    alternate_palettes: None,
                    packet_programs: None,
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
            auxiliary_commands: None,
            auxiliary_archive: None,
            font_bank: None,
                })
                .is_err());
        }
        assert!(extraction
            .validate_files(BaseFiles {
                manifest: &manifest,
                dimensions: (&extraction.dimensions, b"other"),
                background: None,
                palettes: None,
                alternate_palettes: None,
                packet_programs: None,
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
            auxiliary_commands: None,
            auxiliary_archive: None,
            font_bank: None,
            })
            .is_err());
        rom[48] ^= 1;
        assert!(Extraction::from_ranges(&rom, "test".into(), "test", &ranges, &hash).is_err());
        assert!(
            Extraction::from_ranges(&bytes[..47], "test".into(), "test", &ranges, &hash).is_err()
        );
        assert!(
            Extraction::from_ranges(&rom, "test".into(), "test", &[(u32::MAX, "bad")], &hash)
                .is_err()
        );
        assert!(Extraction::from_rom(&rom).is_err());
    }

    #[test]
    #[ignore = "requires the user's exact USA and EUR ROMs; no game data is bundled"]
    fn regional_extractions_are_independent_and_roundtrip() {
        for (env, version, ranges) in [
            ("FFTA_TEST_ROM_USA", Version::Usa, 2),
            ("FFTA_TEST_ROM_EUR", Version::Europe, 2),
        ] {
            let rom = std::fs::read(std::env::var(env).unwrap()).unwrap();
            let extraction = Extraction::from_rom(&rom).unwrap();
            assert_eq!(extraction.version(), version);
            assert_eq!(extraction.manifest.resources[0].sources.len(), ranges);
            for source in &extraction.manifest.resources[0].sources {
                assert_eq!(
                    &rom[source.offset as usize..source.offset as usize + 48],
                    &extraction.dimensions.source_bytes()
                );
            }
            let mut changed = rom;
            changed[0] ^= 1;
            assert!(Extraction::from_rom(&changed).is_err());
        }
    }
}
