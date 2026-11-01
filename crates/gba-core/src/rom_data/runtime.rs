//! Immutable, authenticated base data. Verified OBJ metadata and BG resource
//! ranges are projected onto guest reads; no instruction, timing or DMA is skipped.
use super::*;
mod background;
mod palettes;
mod alternate_palettes;
mod packets;
mod graphics;
mod transfers;
mod texts;
mod text_program;
mod text_catalogs;
mod font_keyboard;
mod obj_lists;
mod obj_animations;
mod obj_custom_graphics;
mod ui_raw_palettes;
mod ui_custom_graphics;
mod item_records;
mod metadata16;
use metadata16::Metadata16Resources;
pub use metadata16::Metadata16Stats;
mod metadata52;
use metadata52::Metadata52Resources;
pub use metadata52::Metadata52Stats;
mod metadata4;
use metadata4::Metadata4Resources;
pub use metadata4::Metadata4Stats;
mod conditional_four_byte_lookup;
use conditional_four_byte_lookup::ConditionalLookupResources;
pub use conditional_four_byte_lookup::ConditionalLookupStats;
mod conditional_packed_lookup;
use conditional_packed_lookup::PackedLookupResources;
pub use conditional_packed_lookup::PackedLookupStats;
mod conditional_descriptor_banks;
use conditional_descriptor_banks::DescriptorBanksResources;
pub use conditional_descriptor_banks::DescriptorBanksStats;
mod conditional_descriptor_bank_zero;
use conditional_descriptor_bank_zero::DescriptorBankZeroResources;
pub use conditional_descriptor_bank_zero::DescriptorBankZeroStats;
mod conditional_property_masks;
use conditional_property_masks::PropertyMasksResources;
pub use conditional_property_masks::PropertyMasksStats;
mod compact_obj_animations;
use compact_obj_animations::CompactObjResources;
pub use compact_obj_animations::CompactObjStats;
mod compact_obj_animations_secondary;
use compact_obj_animations_secondary::SecondaryCompactObjResources;
pub use compact_obj_animations_secondary::SecondaryCompactObjStats;
mod compact_obj_animations_tertiary;
use compact_obj_animations_tertiary::TertiaryCompactObjResources;
pub use compact_obj_animations_tertiary::TertiaryCompactObjStats;
mod compact_obj_animations_quaternary;
use compact_obj_animations_quaternary::QuaternaryCompactObjResources;
pub use compact_obj_animations_quaternary::QuaternaryCompactObjStats;
mod compact_obj_animations_quinary;
use compact_obj_animations_quinary::QuinaryCompactObjResources;
pub use compact_obj_animations_quinary::QuinaryCompactObjStats;
mod compact_obj_animations_senary;
use compact_obj_animations_senary::SenaryCompactObjResources;
pub use compact_obj_animations_senary::SenaryCompactObjStats;
mod metadata2;
use metadata2::Metadata2Resources;
pub use metadata2::Metadata2Stats;
mod default48;
use default48::Default48Resources;
pub use default48::Default48Stats;
mod constructor_alternates;
use constructor_alternates::ConstructorAlternatesResources;
pub use constructor_alternates::ConstructorAlternatesStats;
mod metadata14;
use metadata14::Metadata14Resources;
pub use metadata14::Metadata14Stats;
mod metadata12;
use metadata12::Metadata12Resources;
pub use metadata12::Metadata12Stats;
mod global_pair_flags;
use global_pair_flags::GlobalPairFlagsResources;
pub use global_pair_flags::GlobalPairFlagsStats;
mod evaluation_parameters;
use evaluation_parameters::EvaluationParametersResources;
pub use evaluation_parameters::EvaluationParametersStats;
mod progression_rewards;
use progression_rewards::ProgressionRewardsResources;
pub use progression_rewards::ProgressionRewardsStats;
mod progression_gates;
use progression_gates::ProgressionGatesResources;
pub use progression_gates::ProgressionGatesStats;
mod progression_selection_gates;
use progression_selection_gates::ProgressionSelectionGatesResources;
pub use progression_selection_gates::ProgressionSelectionGatesStats;
mod aggregate_flag_keys;
use aggregate_flag_keys::AggregateFlagKeysResources;
pub use aggregate_flag_keys::AggregateFlagKeysStats;
mod trade_parameters;
use trade_parameters::TradeParametersResources;
pub use trade_parameters::TradeParametersStats;
mod item_effect_parameters;
use item_effect_parameters::ItemEffectParametersResources;
pub use item_effect_parameters::ItemEffectParametersStats;
mod audio_directory;
use audio_directory::AudioDirectoryResources;
pub use audio_directory::AudioDirectoryStats;
mod audio_tones;
use audio_tones::AudioTonesResources;
pub use audio_tones::AudioTonesStats;
mod audio_sequences;
use audio_sequences::AudioSequencesResources;
pub use audio_sequences::AudioSequencesStats;
mod audio_driver;
use audio_driver::AudioDriverResources;
pub use audio_driver::AudioDriverStats;
mod palette_effect_dispatch;
use palette_effect_dispatch::PaletteEffectDispatchResources;
pub use palette_effect_dispatch::PaletteEffectDispatchStats;
mod palette_programs;
use palette_programs::PaletteProgramResources;
pub use palette_programs::PaletteProgramStats;
mod blend_programs;
use blend_programs::BlendProgramResources;
pub use blend_programs::BlendProgramStats;
mod task_descriptors;
use task_descriptors::TaskDescriptorResources;
pub use task_descriptors::TaskDescriptorStats;
mod effect_event_directories;
use effect_event_directories::EffectEventDirectoryResources;
pub use effect_event_directories::EffectEventDirectoryStats;
mod script_commands;
use script_commands::ScriptCommandResources;
pub use script_commands::ScriptCommandStats;
mod script_archive;
use script_archive::ScriptArchiveResources;
pub use script_archive::ScriptArchiveStats;
mod script_text;
use script_text::ScriptTextResources;
pub use script_text::ScriptTextStats;
mod script_coordinates;
use script_coordinates::ScriptCoordinateResources;
pub use script_coordinates::ScriptCoordinateStats;
mod ui_choice_lists;
use ui_choice_lists::UiChoiceListResources;
pub use ui_choice_lists::UiChoiceListStats;
mod unit_obj_initial_pose;
use unit_obj_initial_pose::UnitObjInitialPoseResources;
pub use unit_obj_initial_pose::UnitObjInitialPoseStats;
pub(super) mod color_mode_palettes;
use color_mode_palettes::ColorModePaletteResources;
pub use color_mode_palettes::ColorModePaletteStats;
mod unit_obj_equipment;
use unit_obj_equipment::UnitObjEquipmentResources;
pub use unit_obj_equipment::UnitObjEquipmentStats;
mod unit_obj_normal_battle;
use unit_obj_normal_battle::UnitObjNormalBattleResources;
pub use unit_obj_normal_battle::UnitObjNormalBattleStats;
mod unit_obj_alternate_battle;
use unit_obj_alternate_battle::UnitObjAlternateBattleResources;
pub use unit_obj_alternate_battle::UnitObjAlternateBattleStats;
mod unit_obj_script_special_normal;
use unit_obj_script_special_normal::UnitObjScriptSpecialNormalResources;
pub use unit_obj_script_special_normal::UnitObjScriptSpecialNormalStats;
mod unit_obj_script_special_alternate;
use unit_obj_script_special_alternate::UnitObjScriptSpecialAlternateResources;
pub use unit_obj_script_special_alternate::UnitObjScriptSpecialAlternateStats;
mod unit_obj_script_override;
use unit_obj_script_override::UnitObjScriptOverrideResources;
pub use unit_obj_script_override::UnitObjScriptOverrideStats;
mod unit_obj_script_registry_special;
use unit_obj_script_registry_special::UnitObjScriptRegistrySpecialResources;
pub use unit_obj_script_registry_special::UnitObjScriptRegistrySpecialStats;
mod unit_obj_script_registry_generic;
use unit_obj_script_registry_generic::UnitObjScriptRegistryGenericResources;
pub use unit_obj_script_registry_generic::UnitObjScriptRegistryGenericStats;
mod unit_obj_property_words;
use unit_obj_property_words::UnitObjPropertyWordsResources;
pub use unit_obj_property_words::UnitObjPropertyWordsStats;
mod unit_obj_byte_property_words;
use unit_obj_byte_property_words::UnitObjBytePropertyWordsResources;
pub use unit_obj_byte_property_words::UnitObjBytePropertyWordsStats;
mod unit_obj_ffff_reader_fields;
use unit_obj_ffff_reader_fields::UnitObjFfffReaderFieldsResources;
pub use unit_obj_ffff_reader_fields::UnitObjFfffReaderFieldsStats;
mod slot_job_lookup;
use slot_job_lookup::SlotJobLookupResources;
pub use slot_job_lookup::SlotJobLookupStats;
mod observed_task_parameters;
use observed_task_parameters::ObservedTaskParametersResources;
pub use observed_task_parameters::ObservedTaskParametersStats;
mod constructor_obj_animation;
mod task_cycle_deltas;
use constructor_obj_animation::ConstructorObjAnimationResources;
use task_cycle_deltas::TaskCycleDeltasResources;
pub use constructor_obj_animation::ConstructorObjAnimationStats;
pub use task_cycle_deltas::TaskCycleDeltasStats;
mod ui_raw_transfers;
use ui_raw_transfers::RawTransfersResources;
pub use ui_raw_transfers::RawTransfersStats;
pub use ui_raw_transfers::DynamicFragmentsStats;
mod indexed_graphics;
use indexed_graphics::IndexedGraphicsResources;
pub use indexed_graphics::IndexedGraphicsStats;
mod world_records38;
use world_records38::WorldRecords38Resources;
pub use world_records38::WorldRecords38Stats;
mod parameter_records27;
use parameter_records27::ParameterRecords27Resources;
pub use parameter_records27::ParameterRecords27Stats;
mod event_records12;
use event_records12::EventRecords12Resources;
pub use event_records12::EventRecords12Stats;
mod selection_byte_tables;
use selection_byte_tables::SelectionByteTablesResources;
pub use selection_byte_tables::SelectionByteTablesStats;
mod mission_records70;
use mission_records70::MissionRecords70Resources;
pub use mission_records70::MissionRecords70Stats;
mod conditional_records8;
use conditional_records8::ConditionalRecords8Resources;
pub use conditional_records8::ConditionalRecords8Stats;
mod records4;
use records4::Records4Resources;
pub use records4::Records4Stats;
mod metadata28;
use metadata28::Metadata28Resources;
pub use metadata28::Metadata28Stats;
mod encounter_coordinates;
use encounter_coordinates::EncounterCoordinatesResources;
pub use encounter_coordinates::EncounterCoordinatesStats;
mod encounter_unit_templates;
use encounter_unit_templates::EncounterUnitTemplatesResources;
pub use encounter_unit_templates::EncounterUnitTemplatesStats;
mod encounter_headers;
use encounter_headers::EncounterHeadersResources;
pub use encounter_headers::EncounterHeadersStats;
mod world_roads;
use world_roads::WorldRoadsResources;
pub use world_roads::WorldRoadsStats;
mod world_nodes;
use world_nodes::WorldNodesResources;
pub use world_nodes::WorldNodesStats;
mod word_task_obj_animations;
use word_task_obj_animations::WordTaskObjResources;
pub type WordTaskObjStats = ObjAnimationStats;
mod map_raster_palettes;
use map_raster_palettes::MapRasterPaletteResources;
pub use map_raster_palettes::MapRasterPaletteStats;
mod auxiliary_scripts;
use auxiliary_scripts::AuxiliaryScriptResources;
pub use auxiliary_scripts::{AuxiliaryCommandStats, AuxiliaryArchiveStats};
mod item_links;
use item_links::ItemLinkResources;
pub use item_links::ItemLinkStats;
use item_records::ItemResources;
pub use item_records::ItemStats;
use ui_custom_graphics::UiGraphicsResources;
pub use ui_custom_graphics::{UiGraphicsStats, TableGraphicsStats};
use ui_raw_palettes::UiPaletteResources;
pub use ui_raw_palettes::UiPaletteStats;
use obj_custom_graphics::ObjGraphicsResources;
pub use obj_custom_graphics::ObjGraphicsStats;
use obj_animations::ObjAnimationResources;
pub use obj_animations::ObjAnimationStats;
use obj_lists::ObjListResources;
pub use obj_lists::ObjListStats;
use font_keyboard::FontKeyboardResources;
pub use font_keyboard::FontKeyboardStats;
use background::BackgroundResources;
pub use background::BackgroundStats;
use palettes::PaletteResources;
pub use palettes::{PaletteRead, PaletteStats};
use alternate_palettes::AlternatePaletteResources;
pub use alternate_palettes::AlternatePaletteStats;
use packets::PacketResources;
pub use packets::{PacketRead, PacketStats};
use graphics::GraphicsResources;
pub use graphics::GraphicsStats;
use transfers::TransferResources;
pub use transfers::TransferStats;
use texts::TextResources;
pub use texts::TextStats;
use text_catalogs::TextCatalogResources;
pub use text_catalogs::TextCatalogStats;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceStats {
    pub bus_reads: u64,
    pub bus_source_mask: u8,
    pub host_rom_reads: u64,
    pub host_ram_reads: u64,
    pub dynamic_ram_reads: u64,
    pub validation_reads: u64,
}

pub struct RuntimeResources {
    source_sha256: String,
    dataset_sha256: String,
    file_sha256: String,
    version: Version,
    dimensions: ObjDimensions,
    projection: [u8; 48],
    bus_reads: AtomicU64,
    bus_source_mask: AtomicU64,
    host_rom_reads: AtomicU64,
    host_ram_reads: AtomicU64,
    dynamic_ram_reads: AtomicU64,
    validation_reads: AtomicU64,
    background: Option<Box<BackgroundResources>>,
    palettes: Option<Box<PaletteResources>>,
    alternate_palettes: Option<Box<AlternatePaletteResources>>,
    packet_programs: Option<Box<PacketResources>>,
    main_graphics: Option<Box<GraphicsResources>>,
    transfers: Option<Box<TransferResources>>,
    short_texts: Option<Box<TextResources>>,
    text_catalogs: Option<Box<TextCatalogResources>>,
    font_keyboard: Option<Box<FontKeyboardResources>>,
    obj_lists: Option<Box<ObjListResources>>,
    obj_animations: Option<Box<ObjAnimationResources>>,
    obj_graphics: Option<Box<ObjGraphicsResources>>,
    ui_palettes: Option<Box<UiPaletteResources>>,
    ui_graphics: Option<Box<UiGraphicsResources>>,
    items: Option<Box<ItemResources>>,
    item_links: Option<Box<ItemLinkResources>>,
    metadata16: Option<Box<Metadata16Resources>>,
    metadata52: Option<Box<Metadata52Resources>>,
    metadata4: Option<Box<Metadata4Resources>>,
    metadata2: Option<Box<Metadata2Resources>>,
    default48: Option<Box<Default48Resources>>,
    constructor_alternates: Option<Box<ConstructorAlternatesResources>>,
    metadata14: Option<Box<Metadata14Resources>>,
    metadata12: Option<Box<Metadata12Resources>>,
    global_pair_flags: Option<Box<GlobalPairFlagsResources>>,
    evaluation_parameters: Option<Box<EvaluationParametersResources>>,
    progression_rewards: Option<Box<ProgressionRewardsResources>>,
    progression_gates: Option<Box<ProgressionGatesResources>>,
    progression_selection_gates: Option<Box<ProgressionSelectionGatesResources>>,
    aggregate_flag_keys: Option<Box<AggregateFlagKeysResources>>,
    trade_parameters: Option<Box<TradeParametersResources>>,
    item_effect_parameters: Option<Box<ItemEffectParametersResources>>,
    audio_directory: Option<Box<AudioDirectoryResources>>,
    audio_tones: Option<Box<AudioTonesResources>>,
    audio_sequences: Option<Box<AudioSequencesResources>>,
    audio_driver: Option<Box<AudioDriverResources>>,
    palette_effect_dispatch: Option<Box<PaletteEffectDispatchResources>>,
    palette_programs: Option<Box<PaletteProgramResources>>,
    blend_programs: Option<Box<BlendProgramResources>>,
    task_descriptors: Option<Box<TaskDescriptorResources>>,
    effect_event_directories: Option<Box<EffectEventDirectoryResources>>,
    script_commands: Option<Box<ScriptCommandResources>>,
    script_archive: Option<Box<ScriptArchiveResources>>,
    script_text: Option<Box<ScriptTextResources>>,
    script_coordinates: Option<Box<ScriptCoordinateResources>>,
    ui_choice_lists: Option<Box<UiChoiceListResources>>,
    unit_obj_initial_pose: Option<Box<UnitObjInitialPoseResources>>,
    color_mode_palettes: Option<Box<ColorModePaletteResources>>,
    unit_obj_equipment: Option<Box<UnitObjEquipmentResources>>,
    unit_obj_normal_battle: Option<Box<UnitObjNormalBattleResources>>,
    unit_obj_alternate_battle: Option<Box<UnitObjAlternateBattleResources>>,
    unit_obj_script_special_normal: Option<Box<UnitObjScriptSpecialNormalResources>>,
    unit_obj_script_special_alternate: Option<Box<UnitObjScriptSpecialAlternateResources>>,
    unit_obj_script_override: Option<Box<UnitObjScriptOverrideResources>>,
    unit_obj_script_registry_special: Option<Box<UnitObjScriptRegistrySpecialResources>>,
    unit_obj_script_registry_generic: Option<Box<UnitObjScriptRegistryGenericResources>>,
    unit_obj_property_words: Option<Box<UnitObjPropertyWordsResources>>,
    unit_obj_byte_property_words: Option<Box<UnitObjBytePropertyWordsResources>>,
    unit_obj_ffff_reader_fields: Option<Box<UnitObjFfffReaderFieldsResources>>,
    slot_job_lookup: Option<Box<SlotJobLookupResources>>,
    observed_task_parameters: Option<Box<ObservedTaskParametersResources>>,
    constructor_obj_animation: Option<Box<ConstructorObjAnimationResources>>,
    task_cycle_deltas: Option<Box<TaskCycleDeltasResources>>,
    word_task_obj_animations: Option<Box<WordTaskObjResources>>,
    world_nodes: Option<Box<WorldNodesResources>>,
    world_roads: Option<Box<WorldRoadsResources>>,
    encounter_headers: Option<Box<EncounterHeadersResources>>,
    encounter_unit_templates: Option<Box<EncounterUnitTemplatesResources>>,
    encounter_coordinates: Option<Box<EncounterCoordinatesResources>>,
    metadata28: Option<Box<Metadata28Resources>>,
    records4: Option<Box<Records4Resources>>,
    conditional_records8: Option<Box<ConditionalRecords8Resources>>,
    mission_records70: Option<Box<MissionRecords70Resources>>,
    selection_byte_tables: Option<Box<SelectionByteTablesResources>>,
    event_records12: Option<Box<EventRecords12Resources>>,
    parameter_records27: Option<Box<ParameterRecords27Resources>>,
    world_records38: Option<Box<WorldRecords38Resources>>,
    indexed_graphics: Option<Box<IndexedGraphicsResources>>,
    ui_raw_transfers: Option<Box<RawTransfersResources>>,
    dynamic_raw_fragments: Option<Box<ui_raw_transfers::DynamicFragmentsResources>>,
    conditional_four_byte_lookup: Option<Box<ConditionalLookupResources>>,
    conditional_packed_lookup: Option<Box<PackedLookupResources>>,
    conditional_descriptor_banks: Option<Box<DescriptorBanksResources>>,
    conditional_descriptor_bank_zero: Option<Box<DescriptorBankZeroResources>>,
    conditional_property_masks: Option<Box<PropertyMasksResources>>,
    compact_obj_animations: Option<Box<CompactObjResources>>,
    secondary_compact_obj_animations: Option<Box<SecondaryCompactObjResources>>,
    tertiary_compact_obj_animations: Option<Box<TertiaryCompactObjResources>>,
    quaternary_compact_obj_animations: Option<Box<QuaternaryCompactObjResources>>,
    quinary_compact_obj_animations: Option<Box<QuinaryCompactObjResources>>,
    senary_compact_obj_animations: Option<Box<SenaryCompactObjResources>>,
    map_raster_palettes: Option<Box<MapRasterPaletteResources>>,
    auxiliary_scripts: Option<Box<AuxiliaryScriptResources>>,
}

impl RuntimeResources {
    pub fn conditional_four_byte_lookup_stats(&self) -> Option<ConditionalLookupStats> { self.conditional_four_byte_lookup.as_ref().map(|r| r.stats()) }
    pub fn conditional_packed_lookup_stats(&self) -> Option<PackedLookupStats> { self.conditional_packed_lookup.as_ref().map(|r| r.stats()) }
    pub fn conditional_descriptor_banks_stats(&self) -> Option<DescriptorBanksStats> { self.conditional_descriptor_banks.as_ref().map(|r| r.stats()) }
    pub fn conditional_descriptor_bank_zero_stats(&self) -> Option<DescriptorBankZeroStats> { self.conditional_descriptor_bank_zero.as_ref().map(|r| r.stats()) }
    pub fn conditional_property_masks_stats(&self) -> Option<PropertyMasksStats> { self.conditional_property_masks.as_ref().map(|r| r.stats()) }
    pub fn compact_obj_animations_stats(&self) -> Option<CompactObjStats> { self.compact_obj_animations.as_ref().map(|r| r.stats()) }
    pub fn secondary_compact_obj_animations_stats(&self) -> Option<SecondaryCompactObjStats> { self.secondary_compact_obj_animations.as_ref().map(|r| r.stats()) }
    pub fn tertiary_compact_obj_animations_stats(&self) -> Option<TertiaryCompactObjStats> { self.tertiary_compact_obj_animations.as_ref().map(|r| r.stats()) }
    pub fn quaternary_compact_obj_animations_stats(&self) -> Option<QuaternaryCompactObjStats> { self.quaternary_compact_obj_animations.as_ref().map(|r| r.stats()) }
    pub fn quinary_compact_obj_animations_stats(&self) -> Option<QuinaryCompactObjStats> { self.quinary_compact_obj_animations.as_ref().map(|r| r.stats()) }
    pub fn senary_compact_obj_animations_stats(&self) -> Option<SenaryCompactObjStats> { self.senary_compact_obj_animations.as_ref().map(|r| r.stats()) }
    /// The frontend supplies structures parsed from the very bytes whose file
    /// digest is in the manifest. Reauthenticate the base against the actual
    /// selected image; no editable/mod resource can enter through this API.
    pub fn from_base_files(rom: &[u8], files: BaseFiles<'_>) -> Result<Arc<Self>, String> {
        let source = Extraction::from_rom(rom)?;
        source.validate_files(files)?;
        let manifest = files.manifest;
        let dimensions = files.dimensions.0.clone();
        let background = files.background;
        let file_sha256 = manifest.resources[0].sha256.clone();
        let mut identity = b"FFTA-BASE-RESOURCES-v1\0".to_vec();
        identity.extend_from_slice(manifest.source_rom_sha256.as_bytes());
        identity.extend_from_slice(&manifest.schema.to_le_bytes());
        identity.extend_from_slice(&manifest.extractor_revision.to_le_bytes());
        identity.extend_from_slice(&manifest.decoder_revision.to_le_bytes());
        for resource in &manifest.resources {
            identity.extend_from_slice(resource.id.as_bytes());
            identity.push(0);
            identity.extend_from_slice(resource.sha256.as_bytes());
            identity.push(0);
        }
        let mut resources = Self::new(
            source.manifest.source_rom_sha256.clone(),
            ffta_rom::digest(&identity),
            file_sha256,
            source.version(),
            dimensions,
        );
        if let Some((decoded, _)) = background {
            let root = source.manifest.resources[1].sources[0].offset as usize;
            resources.background = Some(Box::new(BackgroundResources::new(
                root,
                decoded.clone(),
                manifest.resources[1].sha256.clone(),
            )?));
        }
        if let Some((decoded, _)) = files.palettes {
            resources.palettes = Some(Box::new(PaletteResources::new(
                decoded.clone(),
                &source.palettes.as_ref().unwrap().sources,
                manifest.resources[2].sha256.clone(),
            )?));
        }
        if let Some((decoded, _)) = files.alternate_palettes {
            resources.alternate_palettes = Some(Box::new(AlternatePaletteResources::new(
                decoded.clone(), source.alternate_palettes.as_ref().unwrap(),
                manifest.resources[3].sha256.clone(),
            )?));
        }
        // USA stays authenticated but verified-only until its consumer checks.
        if source.version() == Version::Europe {
            if let Some((decoded, _)) = files.packet_programs {
                resources.packet_programs = Some(Box::new(PacketResources::new(
                    decoded.clone(), &source.packet_programs.as_ref().unwrap().sources,
                    manifest.resources[4].sha256.clone(),
                )?));
            }
            if let Some((decoded, _)) = files.main_graphics {
                resources.main_graphics = Some(Box::new(GraphicsResources::new(
                    decoded.clone(), &source.main_graphics.as_ref().unwrap().sources,
                    manifest.resources[5].sha256.clone(),
                )?));
            }
            if let Some((decoded, _)) = files.transfers {
                resources.transfers = Some(Box::new(TransferResources::new(
                    decoded.clone(), &source.transfers.as_ref().unwrap().sources,
                    manifest.resources[6].sha256.clone(),
                )?));
            }
        }
        if let Some((decoded, _)) = files.short_texts.filter(|_| files.text_catalogs.is_none()) {
            resources.short_texts = Some(Box::new(TextResources::new(
                decoded.clone(), &source.short_texts.as_ref().unwrap().sources,
                manifest.resources[7].sha256.clone(),
            )?));
        }
        if let Some((decoded, _)) = files.text_catalogs {
            resources.text_catalogs = Some(Box::new(TextCatalogResources::new(
                decoded.clone(), &source.text_catalogs.as_ref().unwrap().sources,
                manifest.resources[8].sha256.clone(),
            )?));
        }
        if let (Some((font, _)), Some((keyboard, _))) = (files.font_bank, files.name_keyboard) {
            resources.font_keyboard = Some(Box::new(FontKeyboardResources::new(font.clone(), keyboard.clone())?));
            resources.text_catalogs = resources.text_catalogs.take().map(|t| Box::new(t.without_font()));
        }
        if let Some((lists, _)) = files.obj_lists {
            resources.obj_lists = Some(Box::new(ObjListResources::new(lists.clone())?));
        }
        if let Some((banks, _)) = files.obj_animations {
            resources.obj_animations = Some(Box::new(ObjAnimationResources::new(banks.clone())?));
        }
        if let Some((graphics, _)) = files.obj_graphics {
            resources.obj_graphics = Some(Box::new(ObjGraphicsResources::new(
                graphics.clone(), &source.obj_graphics.as_ref().unwrap().sources,
                manifest.resources[13].sha256.clone(),
            )?));
        }
        if let Some((palettes, _)) = files.ui_palettes {
            resources.ui_palettes = Some(Box::new(UiPaletteResources::new(
                palettes.clone(), &source.ui_palettes.as_ref().unwrap().sources,
                manifest.resources[14].sha256.clone(),
            )?));
        }
        if let Some((graphics, _)) = files.ui_graphics {
            resources.ui_graphics = Some(Box::new(UiGraphicsResources::new(
                graphics.clone(), &source.ui_graphics.as_ref().unwrap().sources,
                files.obj_graphics.ok_or("Missing shared OBJ bank owner")?.0,
                manifest.resources[15].sha256.clone(),
            )?));
        }
        if let Some((items,_))= files.items {
            resources.items = Some(Box::new(ItemResources::new(items.clone(), &source.items.as_ref().unwrap().sources, manifest.resources[16].sha256.clone())?));
        }
        if let Some((links,_))= files.item_links {
            resources.item_links = Some(Box::new(ItemLinkResources::new(links.clone(), &source.item_links.as_ref().unwrap().sources, files.items.ok_or("Missing item-link owner")?.0, manifest.resources[17].sha256.clone())?));
        }
        if let Some((data,_))= files.metadata16 {
            resources.metadata16 = Some(Box::new(Metadata16Resources::new(data.clone(), &source.metadata16.as_ref().unwrap().sources, manifest.resources[18].sha256.clone())?));
        }
        if let Some((data,_))= files.metadata52 {
            resources.metadata52 = Some(Box::new(Metadata52Resources::new(data.clone(), &source.metadata52.as_ref().unwrap().sources, files.item_links.ok_or("Missing metadata52 link owner")?.0)?));
        }
        if let Some((data,_))= files.metadata4 {
            resources.metadata4 = Some(Box::new(Metadata4Resources::new(data.clone(), &source.metadata4.as_ref().unwrap().sources, files.metadata52.ok_or("Missing metadata4 owner")?.0)?));
        }
        if let Some((data,_))= files.metadata2 {
            resources.metadata2 = Some(Box::new(Metadata2Resources::new(data.clone(), &source.metadata2.as_ref().unwrap().sources, files.metadata52.ok_or("Missing metadata2 owner")?.0)?));
        }
        if let Some((data,_))= files.default48 {
            resources.default48 = Some(Box::new(Default48Resources::new(data.clone(), &source.default48.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.constructor_alternates {
            resources.constructor_alternates = Some(Box::new(ConstructorAlternatesResources::new(data.clone(), &source.constructor_alternates.as_ref().unwrap().sources, files.items.ok_or("Missing constructor-alternate item owner")?.0)?));
        }
        if let Some((data,_))= files.metadata14 {
            resources.metadata14 = Some(Box::new(Metadata14Resources::new(data.clone(), &source.metadata14.as_ref().unwrap().sources, files.metadata52.ok_or("Missing metadata14 job owner")?.0)?));
        }
        if let Some((data,_))= files.metadata12 {
            resources.metadata12 = Some(Box::new(Metadata12Resources::new(data.clone(), &source.metadata12.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.global_pair_flags {
            resources.global_pair_flags = Some(Box::new(GlobalPairFlagsResources::new(data.clone(), &source.global_pair_flags.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.evaluation_parameters {
            resources.evaluation_parameters = Some(Box::new(EvaluationParametersResources::new(data.clone(), &source.evaluation_parameters.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.progression_rewards {
            resources.progression_rewards = Some(Box::new(ProgressionRewardsResources::new(data.clone(), &source.progression_rewards.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.progression_gates {
            resources.progression_gates = Some(Box::new(ProgressionGatesResources::new(data.clone(), &source.progression_gates.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.progression_selection_gates {
            resources.progression_selection_gates = Some(Box::new(ProgressionSelectionGatesResources::new(data.clone(), &source.progression_selection_gates.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.aggregate_flag_keys {
            resources.aggregate_flag_keys = Some(Box::new(AggregateFlagKeysResources::new(data.clone(), &source.aggregate_flag_keys.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.trade_parameters {
            resources.trade_parameters = Some(Box::new(TradeParametersResources::new(data.clone(), &source.trade_parameters.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.item_effect_parameters {
            resources.item_effect_parameters = Some(Box::new(ItemEffectParametersResources::new(data.clone(), &source.item_effect_parameters.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.audio_directory {
            resources.audio_directory = Some(Box::new(AudioDirectoryResources::new(data.clone(), &source.audio_directory.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.audio_tones {
            resources.audio_tones = Some(Box::new(AudioTonesResources::new(data.clone(), &source.audio_tones.as_ref().unwrap().sources, files.audio_directory.ok_or("Missing typed audio directory owner")?.0)?));
        }
        if let Some((data,_))= files.audio_sequences {
            resources.audio_sequences = Some(Box::new(AudioSequencesResources::new(data.clone(), &source.audio_sequences.as_ref().unwrap().sources, files.audio_directory.ok_or("Missing typed audio directory owner")?.0, files.audio_tones.ok_or("Missing typed audio tones owner")?.0)?));
        }
        if let Some((data,_))= files.audio_driver {
            resources.audio_driver = Some(Box::new(AudioDriverResources::new(data.clone(), &source.audio_driver.as_ref().unwrap().sources, files.audio_sequences.ok_or("Missing typed audio sequence owner")?.0)?));
        }
        if let Some((data,_))= files.palette_effect_dispatch {
            resources.palette_effect_dispatch = Some(Box::new(PaletteEffectDispatchResources::new(data.clone(), &source.palette_effect_dispatch.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.palette_programs {
            resources.palette_programs = Some(Box::new(PaletteProgramResources::new(data.clone(), &source.palette_programs.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.blend_programs {
            resources.blend_programs = Some(Box::new(BlendProgramResources::new(data.clone(), &source.blend_programs.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.task_descriptors {
            resources.task_descriptors = Some(Box::new(TaskDescriptorResources::new(data.clone(), &source.task_descriptors.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.effect_event_directories {
            resources.effect_event_directories = Some(Box::new(EffectEventDirectoryResources::new(data.clone(), &source.effect_event_directories.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.script_commands {
            resources.script_commands = Some(Box::new(ScriptCommandResources::new(data.clone(), &source.script_commands.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.script_archive {
            resources.script_archive = Some(Box::new(ScriptArchiveResources::new(data.clone(), &source.script_archive.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.script_text {
            resources.script_text = Some(Box::new(ScriptTextResources::new(data.clone(), &source.script_text.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.script_coordinates {
            resources.script_coordinates = Some(Box::new(ScriptCoordinateResources::new(data.clone(), &source.script_coordinates.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.ui_choice_lists {
            resources.ui_choice_lists = Some(Box::new(UiChoiceListResources::new(data.clone(), &source.ui_choice_lists.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.unit_obj_initial_pose {
            resources.unit_obj_initial_pose = Some(Box::new(UnitObjInitialPoseResources::new(data.clone(), &source.unit_obj_initial_pose.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.color_mode_palettes {
            resources.color_mode_palettes = Some(Box::new(ColorModePaletteResources::new(data.clone(), &source.color_mode_palettes.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.unit_obj_equipment {
            resources.unit_obj_equipment = Some(Box::new(UnitObjEquipmentResources::new(data.clone(), &source.unit_obj_equipment.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.unit_obj_normal_battle {
            resources.unit_obj_normal_battle = Some(Box::new(UnitObjNormalBattleResources::new(data.clone(), &source.unit_obj_normal_battle.as_ref().unwrap().sources, files.unit_obj_initial_pose.unwrap().0, files.unit_obj_equipment.unwrap().0)?));
        }
        if let Some((data,_))= files.unit_obj_alternate_battle {
            resources.unit_obj_alternate_battle = Some(Box::new(UnitObjAlternateBattleResources::new(data.clone(), &source.unit_obj_alternate_battle.as_ref().unwrap().sources, files.unit_obj_initial_pose.unwrap().0, files.unit_obj_equipment.unwrap().0, files.unit_obj_normal_battle.unwrap().0)?));
        }
        if let Some((data,_))= files.unit_obj_script_special_normal {
            resources.unit_obj_script_special_normal = Some(Box::new(UnitObjScriptSpecialNormalResources::new(data.clone(), &source.unit_obj_script_special_normal.as_ref().unwrap().sources, files.unit_obj_initial_pose.unwrap().0, files.unit_obj_normal_battle.unwrap().0)?));
        }
        if let Some((data,_))= files.unit_obj_script_special_alternate {
            resources.unit_obj_script_special_alternate = Some(Box::new(UnitObjScriptSpecialAlternateResources::new(data.clone(), &source.unit_obj_script_special_alternate.as_ref().unwrap().sources, files.unit_obj_initial_pose.unwrap().0, files.unit_obj_normal_battle.unwrap().0, files.unit_obj_alternate_battle.unwrap().0)?));
        }
        if let Some((data,_))= files.unit_obj_script_override {
            resources.unit_obj_script_override = Some(Box::new(UnitObjScriptOverrideResources::new(data.clone(), &source.unit_obj_script_override.as_ref().unwrap().sources, files.unit_obj_normal_battle.unwrap().0, files.unit_obj_alternate_battle.unwrap().0)?));
        }
        if let Some((data,_))= files.unit_obj_script_registry_special {
            resources.unit_obj_script_registry_special = Some(Box::new(UnitObjScriptRegistrySpecialResources::new(data.clone(), &source.unit_obj_script_registry_special.as_ref().unwrap().sources, files.unit_obj_initial_pose.unwrap().0, files.unit_obj_equipment.unwrap().0, files.unit_obj_normal_battle.unwrap().0, files.unit_obj_alternate_battle.unwrap().0, files.unit_obj_script_special_normal.unwrap().0, files.unit_obj_script_special_alternate.unwrap().0)?));
        }
        if let Some((data,_))= files.unit_obj_script_registry_generic {
            resources.unit_obj_script_registry_generic = Some(Box::new(UnitObjScriptRegistryGenericResources::new(data.clone(), &source.unit_obj_script_registry_generic.as_ref().unwrap().sources, files.unit_obj_initial_pose.unwrap().0, files.unit_obj_equipment.unwrap().0, files.unit_obj_normal_battle.unwrap().0, files.unit_obj_alternate_battle.unwrap().0, files.unit_obj_script_special_alternate.unwrap().0, files.unit_obj_script_registry_special.unwrap().0)?));
        }
        if let Some((data,_))= files.unit_obj_property_words {
            resources.unit_obj_property_words = Some(Box::new(UnitObjPropertyWordsResources::new(data.clone(), &source.unit_obj_property_words.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.unit_obj_byte_property_words {
            resources.unit_obj_byte_property_words = Some(Box::new(UnitObjBytePropertyWordsResources::new(data.clone(), &source.unit_obj_byte_property_words.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.unit_obj_ffff_reader_fields {
            resources.unit_obj_ffff_reader_fields = Some(Box::new(UnitObjFfffReaderFieldsResources::new(data.clone(), &source.unit_obj_ffff_reader_fields.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.slot_job_lookup {
            resources.slot_job_lookup = Some(Box::new(SlotJobLookupResources::new(data.clone(), &source.slot_job_lookup.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.observed_task_parameters {
            resources.observed_task_parameters = Some(Box::new(ObservedTaskParametersResources::new(data.clone(), &source.observed_task_parameters.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.constructor_obj_animation {
            resources.constructor_obj_animation = Some(Box::new(ConstructorObjAnimationResources::new(data.clone(), &source.constructor_obj_animation.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.task_cycle_deltas {
            resources.task_cycle_deltas = Some(Box::new(TaskCycleDeltasResources::new(data.clone(), &source.task_cycle_deltas.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.word_task_obj_animations {
            resources.word_task_obj_animations = Some(Box::new(WordTaskObjResources::new(data.clone(), &source.word_task_obj_animations.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.world_nodes {
            resources.world_nodes = Some(Box::new(WorldNodesResources::new(data.clone(), &source.world_nodes.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.world_roads {
            resources.world_roads = Some(Box::new(WorldRoadsResources::new(data.clone(), &source.world_roads.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.encounter_headers {
            resources.encounter_headers = Some(Box::new(EncounterHeadersResources::new(data.clone(), &source.encounter_headers.as_ref().unwrap().sources, files.constructor_alternates.ok_or("Missing encounter constructor dependency")?.0)?));
        }
        if let Some((data,_))= files.encounter_unit_templates {
            resources.encounter_unit_templates = Some(Box::new(EncounterUnitTemplatesResources::new(data.clone(), &source.encounter_unit_templates.as_ref().unwrap().sources, files.encounter_headers.ok_or("Missing encounter header dependency")?.0, files.constructor_alternates.ok_or("Missing encounter constructor dependency")?.0, files.items.ok_or("Missing encounter item dependency")?.0, files.metadata14.ok_or("Missing encounter descriptor dependency")?.0, files.metadata52.ok_or("Missing encounter job dependency")?.0)?));
        }
        if let Some((data,_))=files.encounter_coordinates {
            resources.encounter_coordinates=Some(Box::new(EncounterCoordinatesResources::new(data.clone(),&source.encounter_coordinates.as_ref().unwrap().sources,files.encounter_headers.ok_or("Missing coordinate header dependency")?.0)?));
        }
        if let Some((data,_))=files.metadata28 {
            resources.metadata28=Some(Box::new(Metadata28Resources::new(data.clone(),&source.metadata28.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.records4 {
            resources.records4=Some(Box::new(Records4Resources::new(data.clone(),&source.records4.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.conditional_records8 {
            resources.conditional_records8=Some(Box::new(ConditionalRecords8Resources::new(data.clone(),&source.conditional_records8.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.mission_records70 {
            resources.mission_records70=Some(Box::new(MissionRecords70Resources::new(data.clone(),&source.mission_records70.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.selection_byte_tables {
            resources.selection_byte_tables=Some(Box::new(SelectionByteTablesResources::new(data.clone(),&source.selection_byte_tables.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.event_records12 {
            resources.event_records12=Some(Box::new(EventRecords12Resources::new(data.clone(),&source.event_records12.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.parameter_records27 {
            resources.parameter_records27=Some(Box::new(ParameterRecords27Resources::new(data.clone(),&source.parameter_records27.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.world_records38 {
            resources.world_records38=Some(Box::new(WorldRecords38Resources::new(data.clone(),&source.world_records38.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.indexed_graphics {
            resources.indexed_graphics=Some(Box::new(IndexedGraphicsResources::new(data.clone(),&source.indexed_graphics.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.ui_raw_transfers {
            resources.ui_raw_transfers=Some(Box::new(RawTransfersResources::new(data.clone(),&source.ui_raw_transfers.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.dynamic_raw_fragments {
            resources.dynamic_raw_fragments=Some(Box::new(ui_raw_transfers::DynamicFragmentsResources::new(data.clone(),&source.dynamic_raw_fragments.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.conditional_four_byte_lookup {
            resources.conditional_four_byte_lookup=Some(Box::new(ConditionalLookupResources::new(data.clone(),&source.conditional_four_byte_lookup.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.conditional_packed_lookup {
            resources.conditional_packed_lookup=Some(Box::new(PackedLookupResources::new(data.clone(),&source.conditional_packed_lookup.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.conditional_descriptor_banks {
            resources.conditional_descriptor_banks=Some(Box::new(DescriptorBanksResources::new(data.clone(),&source.conditional_descriptor_banks.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.conditional_descriptor_bank_zero {
            resources.conditional_descriptor_bank_zero=Some(Box::new(DescriptorBankZeroResources::new(data.clone(),&source.conditional_descriptor_bank_zero.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.conditional_property_masks {
            resources.conditional_property_masks=Some(Box::new(PropertyMasksResources::new(data.clone(),&source.conditional_property_masks.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.compact_obj_animations {
            resources.compact_obj_animations=Some(Box::new(CompactObjResources::new(data.clone(),&source.compact_obj_animations.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.secondary_compact_obj_animations {
            resources.secondary_compact_obj_animations=Some(Box::new(SecondaryCompactObjResources::new(data.clone(),&source.secondary_compact_obj_animations.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.tertiary_compact_obj_animations {
            resources.tertiary_compact_obj_animations=Some(Box::new(TertiaryCompactObjResources::new(data.clone(),&source.tertiary_compact_obj_animations.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.quaternary_compact_obj_animations {
            resources.quaternary_compact_obj_animations=Some(Box::new(QuaternaryCompactObjResources::new(data.clone(),&source.quaternary_compact_obj_animations.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.quinary_compact_obj_animations {
            resources.quinary_compact_obj_animations=Some(Box::new(QuinaryCompactObjResources::new(data.clone(),&source.quinary_compact_obj_animations.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))=files.senary_compact_obj_animations {
            resources.senary_compact_obj_animations=Some(Box::new(SenaryCompactObjResources::new(data.clone(),&source.senary_compact_obj_animations.as_ref().unwrap().sources)?));
        }
        if let Some((data,_))= files.map_raster_palettes {
            resources.map_raster_palettes = Some(Box::new(MapRasterPaletteResources::new(data.clone(), &source.map_raster_palettes.as_ref().unwrap().sources)?));
        }
        if let (Some((data,_)),Some((commands,_)))= (files.auxiliary_archive,files.auxiliary_commands) {
            resources.auxiliary_scripts = Some(Box::new(AuxiliaryScriptResources::new(data.clone(),commands.clone(), &source.auxiliary_scripts.as_ref().unwrap().sources)?));
        }
        Ok(Arc::new(resources))
    }

    fn new(
        source_sha256: String,
        dataset_sha256: String,
        file_sha256: String,
        version: Version,
        dimensions: ObjDimensions,
    ) -> Self {
        let projection = dimensions.source_bytes();
        Self {
            source_sha256,
            dataset_sha256,
            file_sha256,
            version,
            dimensions,
            projection,
            bus_reads: AtomicU64::new(0),
            bus_source_mask: AtomicU64::new(0),
            host_rom_reads: AtomicU64::new(0),
            host_ram_reads: AtomicU64::new(0),
            dynamic_ram_reads: AtomicU64::new(0),
            validation_reads: AtomicU64::new(0),
            background: None,
            palettes: None,
            alternate_palettes: None,
            packet_programs: None,
            main_graphics: None,
            transfers: None,
            short_texts: None,
            text_catalogs: None,
            font_keyboard: None,
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
        }
    }

    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn dataset_sha256(&self) -> &str {
        &self.dataset_sha256
    }
    pub fn file_sha256(&self) -> &str {
        &self.file_sha256
    }
    pub fn resource_id(&self) -> &'static str {
        OBJ_DIMENSIONS_ID
    }

    /// Shared/OBJ session allocation, excluding Arc's two reference counters.
    pub fn obj_resident_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.source_sha256.capacity()
            + self.dataset_sha256.capacity()
            + self.file_sha256.capacity()
    }

    pub fn background_resident_bytes(&self) -> usize {
        self.background.as_ref().map_or(0, |bg| bg.resident_bytes())
    }

    /// Complete owned session allocation, excluding the shared Arc headers.
    pub fn resident_bytes(&self) -> usize {
        self.obj_resident_bytes() + self.background_resident_bytes() + self.palette_resident_bytes()
            + self.alternate_palette_resident_bytes()
            + self.packet_resident_bytes()
            + self.graphics_resident_bytes()
            + self.transfer_resident_bytes()
            + self.text_resident_bytes()
            + self.text_catalog_resident_bytes()
            + self.font_keyboard.as_ref().map_or(0, |f| f.resident_bytes())
            + self.obj_lists.as_ref().map_or(0, |l| l.resident_bytes())
            + self.obj_animations.as_ref().map_or(0, |a| a.resident_bytes())
            + self.obj_graphics.as_ref().map_or(0, |g| g.resident_bytes())
            + self.ui_palettes.as_ref().map_or(0, |p| p.resident_bytes())
            + self.ui_graphics.as_ref().map_or(0, |g| g.resident_bytes())
            + self.items.as_ref().map_or(0, |i| i.resident_bytes())
            + self.item_links.as_ref().map_or(0, |i| i.resident_bytes())
            + self.metadata16.as_ref().map_or(0, |i| i.resident_bytes())
            + self.metadata52.as_ref().map_or(0, |i| i.resident_bytes())
            + self.metadata4.as_ref().map_or(0, |i| i.resident_bytes())
            + self.metadata2.as_ref().map_or(0, |i| i.resident_bytes())
            + self.default48.as_ref().map_or(0, |i| i.resident_bytes())
            + self.constructor_alternates.as_ref().map_or(0, |i| i.resident_bytes())
            + self.metadata14.as_ref().map_or(0, |i| i.resident_bytes())
            + self.metadata12.as_ref().map_or(0, |i| i.resident_bytes())
            + self.global_pair_flags.as_ref().map_or(0, |i| i.resident_bytes())
            + self.evaluation_parameters.as_ref().map_or(0, |i| i.resident_bytes())
            + self.progression_rewards.as_ref().map_or(0, |i| i.resident_bytes())
            + self.progression_gates.as_ref().map_or(0, |i| i.resident_bytes())
            + self.progression_selection_gates.as_ref().map_or(0, |i| i.resident_bytes())
            + self.aggregate_flag_keys.as_ref().map_or(0, |i| i.resident_bytes())
            + self.trade_parameters.as_ref().map_or(0, |i| i.resident_bytes())
            + self.item_effect_parameters.as_ref().map_or(0, |i| i.resident_bytes())
            + self.audio_directory.as_ref().map_or(0, |i| i.resident_bytes())
            + self.audio_tones.as_ref().map_or(0, |i| i.resident_bytes())
            + self.audio_sequences.as_ref().map_or(0, |i| i.resident_bytes())
            + self.audio_driver.as_ref().map_or(0, |i| i.resident_bytes())
            + self.palette_effect_dispatch.as_ref().map_or(0, |i| i.resident_bytes())
            + self.palette_programs.as_ref().map_or(0, |i| i.resident_bytes())
            + self.blend_programs.as_ref().map_or(0, |i| i.resident_bytes())
            + self.task_descriptors.as_ref().map_or(0, |i| i.resident_bytes())
            + self.effect_event_directories.as_ref().map_or(0, |i| i.resident_bytes())
            + self.script_commands.as_ref().map_or(0, |i| i.resident_bytes())
            + self.script_archive.as_ref().map_or(0, |i| i.resident_bytes())
            + self.script_text.as_ref().map_or(0, |i| i.resident_bytes())
            + self.script_coordinates.as_ref().map_or(0, |i| i.resident_bytes())
            + self.ui_choice_lists.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_initial_pose.as_ref().map_or(0, |i| i.resident_bytes())
            + self.color_mode_palettes.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_equipment.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_normal_battle.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_alternate_battle.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_script_special_normal.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_script_special_alternate.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_script_override.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_script_registry_special.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_script_registry_generic.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_property_words.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_byte_property_words.as_ref().map_or(0, |i| i.resident_bytes())
            + self.unit_obj_ffff_reader_fields.as_ref().map_or(0, |i| i.resident_bytes())
            + self.slot_job_lookup.as_ref().map_or(0, |i| i.resident_bytes())
            + self.observed_task_parameters.as_ref().map_or(0, |i| i.resident_bytes())
            + self.constructor_obj_animation.as_ref().map_or(0, |i| i.resident_bytes())
            + self.task_cycle_deltas.as_ref().map_or(0, |i| i.resident_bytes())
            + self.word_task_obj_animations.as_ref().map_or(0, |i| i.resident_bytes())
            + self.world_nodes.as_ref().map_or(0, |i| i.resident_bytes())
            + self.world_roads.as_ref().map_or(0, |i| i.resident_bytes())
            + self.encounter_headers.as_ref().map_or(0, |i| i.resident_bytes())
            + self.encounter_unit_templates.as_ref().map_or(0, |i| i.resident_bytes())
            + self.encounter_coordinates.as_ref().map_or(0, |i| i.resident_bytes())
            + self.metadata28.as_ref().map_or(0, |i| i.resident_bytes())
            + self.records4.as_ref().map_or(0, |i| i.resident_bytes())
            + self.conditional_records8.as_ref().map_or(0, |i| i.resident_bytes())
            + self.mission_records70.as_ref().map_or(0, |i| i.resident_bytes())
            + self.selection_byte_tables.as_ref().map_or(0, |i| i.resident_bytes())
            + self.event_records12.as_ref().map_or(0, |i| i.resident_bytes())
            + self.parameter_records27.as_ref().map_or(0, |i| i.resident_bytes())
            + self.world_records38.as_ref().map_or(0, |i| i.resident_bytes())
            + self.indexed_graphics.as_ref().map_or(0, |i| i.resident_bytes())
            + self.ui_raw_transfers.as_ref().map_or(0, |i| i.resident_bytes())
            + self.dynamic_raw_fragments.as_ref().map_or(0, |i| i.resident_bytes())
            + self.conditional_four_byte_lookup.as_ref().map_or(0, |i| i.resident_bytes())
            + self.conditional_packed_lookup.as_ref().map_or(0, |i| i.resident_bytes())
            + self.conditional_descriptor_banks.as_ref().map_or(0, |i| i.resident_bytes())
            + self.conditional_descriptor_bank_zero.as_ref().map_or(0, |i| i.resident_bytes())
            + self.conditional_property_masks.as_ref().map_or(0, |i| i.resident_bytes())
            + self.compact_obj_animations.as_ref().map_or(0, |i| i.resident_bytes())
            + self.secondary_compact_obj_animations.as_ref().map_or(0, |i| i.resident_bytes())
            + self.tertiary_compact_obj_animations.as_ref().map_or(0, |i| i.resident_bytes())
            + self.quaternary_compact_obj_animations.as_ref().map_or(0, |i| i.resident_bytes())
            + self.quinary_compact_obj_animations.as_ref().map_or(0, |i| i.resident_bytes())
            + self.senary_compact_obj_animations.as_ref().map_or(0, |i| i.resident_bytes())
            + self.map_raster_palettes.as_ref().map_or(0, |i| i.resident_bytes())
            + self.auxiliary_scripts.as_ref().map_or(0, |i| i.resident_bytes())
    }

    pub fn background_file_sha256(&self) -> Option<&str> {
        self.background.as_ref().map(|bg| bg.file_sha256())
    }

    pub fn background_stats(&self) -> Option<BackgroundStats> {
        self.background.as_ref().map(|bg| bg.stats())
    }

    pub fn palette_resident_bytes(&self) -> usize {
        self.palettes
            .as_ref()
            .map_or(0, |palettes| palettes.resident_bytes())
    }
    pub fn palette_file_sha256(&self) -> Option<&str> {
        self.palettes
            .as_ref()
            .map(|palettes| palettes.file_sha256())
    }
    pub fn palette_stats(&self) -> Option<PaletteStats> {
        self.palettes.as_ref().map(|palettes| palettes.stats())
    }
    pub fn alternate_palette_resident_bytes(&self) -> usize {
        self.alternate_palettes.as_ref().map_or(0, |palettes| palettes.resident_bytes())
    }
    pub fn alternate_palette_file_sha256(&self) -> Option<&str> {
        self.alternate_palettes.as_ref().map(|palettes| palettes.file_sha256())
    }
    pub fn alternate_palette_stats(&self) -> Option<AlternatePaletteStats> {
        self.alternate_palettes.as_ref().map(|palettes| palettes.stats())
    }
    pub fn packet_resident_bytes(&self) -> usize {
        self.packet_programs.as_ref().map_or(0, |packets| packets.resident_bytes())
    }
    pub fn packet_file_sha256(&self) -> Option<&str> {
        self.packet_programs.as_ref().map(|packets| packets.file_sha256())
    }
    pub fn packet_stats(&self) -> Option<PacketStats> {
        self.packet_programs.as_ref().map(|packets| packets.stats())
    }

    pub fn graphics_resident_bytes(&self) -> usize {
        self.main_graphics.as_ref().map_or(0, |graphics| graphics.resident_bytes())
    }
    pub fn graphics_file_sha256(&self) -> Option<&str> {
        self.main_graphics.as_ref().map(|graphics| graphics.file_sha256())
    }
    pub fn graphics_stats(&self) -> Option<GraphicsStats> {
        self.main_graphics.as_ref().map(|graphics| graphics.stats())
    }

    pub fn transfer_resident_bytes(&self) -> usize {
        self.transfers.as_ref().map_or(0, |t| t.resident_bytes())
    }
    pub fn transfer_file_sha256(&self) -> Option<&str> {
        self.transfers.as_ref().map(|t| t.file_sha256())
    }
    pub fn transfer_stats(&self) -> Option<TransferStats> {
        self.transfers.as_ref().map(|t| t.stats())
    }

    pub fn text_resident_bytes(&self) -> usize { self.short_texts.as_ref().map_or(0, |t| t.resident_bytes()) }
    pub fn text_file_sha256(&self) -> Option<&str> { self.short_texts.as_ref().map(|t| t.file_sha256()) }
    pub fn text_stats(&self) -> Option<TextStats> { self.short_texts.as_ref().map(|t| t.stats()) }

    pub fn text_catalog_resident_bytes(&self) -> usize { self.text_catalogs.as_ref().map_or(0, |t| t.resident_bytes()) }
    pub fn text_catalog_file_sha256(&self) -> Option<&str> { self.text_catalogs.as_ref().map(|t| t.file_sha256()) }
    pub fn text_catalog_stats(&self) -> Option<TextCatalogStats> { self.text_catalogs.as_ref().map(|t| t.stats()) }

    pub fn metadata16_stats(&self) -> Option<Metadata16Stats> {self.metadata16.as_ref().map(|m|m.stats())}
    pub fn metadata52_stats(&self) -> Option<Metadata52Stats> {self.metadata52.as_ref().map(|m|m.stats())}
    pub fn metadata4_stats(&self) -> Option<Metadata4Stats> {self.metadata4.as_ref().map(|m|m.stats())}
    pub fn metadata2_stats(&self) -> Option<Metadata2Stats> {self.metadata2.as_ref().map(|m|m.stats())}
    pub fn default48_stats(&self) -> Option<Default48Stats> {self.default48.as_ref().map(|m|m.stats())}
    pub fn constructor_alternates_stats(&self) -> Option<ConstructorAlternatesStats> {self.constructor_alternates.as_ref().map(|m|m.stats())}
    pub fn metadata14_stats(&self) -> Option<Metadata14Stats> {self.metadata14.as_ref().map(|m|m.stats())}
    pub fn metadata12_stats(&self) -> Option<Metadata12Stats> {self.metadata12.as_ref().map(|m|m.stats())}
    pub fn global_pair_flags_stats(&self) -> Option<GlobalPairFlagsStats> {self.global_pair_flags.as_ref().map(|m|m.stats())}
    pub fn evaluation_parameters_stats(&self) -> Option<EvaluationParametersStats> {self.evaluation_parameters.as_ref().map(|m|m.stats())}
    pub fn progression_rewards_stats(&self) -> Option<ProgressionRewardsStats> {self.progression_rewards.as_ref().map(|m|m.stats())}
    pub fn progression_gates_stats(&self) -> Option<ProgressionGatesStats> {self.progression_gates.as_ref().map(|m|m.stats())}
    pub fn progression_selection_gates_stats(&self) -> Option<ProgressionSelectionGatesStats> {self.progression_selection_gates.as_ref().map(|m|m.stats())}
    pub fn aggregate_flag_keys_stats(&self) -> Option<AggregateFlagKeysStats> {self.aggregate_flag_keys.as_ref().map(|m|m.stats())}
    pub fn trade_parameters_stats(&self) -> Option<TradeParametersStats> {self.trade_parameters.as_ref().map(|m|m.stats())}
    pub fn item_effect_parameters_stats(&self) -> Option<ItemEffectParametersStats> {self.item_effect_parameters.as_ref().map(|m|m.stats())}
    pub fn audio_directory_stats(&self) -> Option<AudioDirectoryStats> {self.audio_directory.as_ref().map(|m|m.stats())}
    pub fn audio_tones_stats(&self) -> Option<AudioTonesStats> {self.audio_tones.as_ref().map(|m|m.stats())}
    pub fn audio_sequences_stats(&self) -> Option<AudioSequencesStats> {self.audio_sequences.as_ref().map(|m|m.stats())}
    pub fn audio_driver_stats(&self) -> Option<AudioDriverStats> {self.audio_driver.as_ref().map(|m|m.stats())}
    pub fn palette_effect_dispatch_stats(&self) -> Option<PaletteEffectDispatchStats> {self.palette_effect_dispatch.as_ref().map(|m|m.stats())}
    pub fn palette_programs_stats(&self) -> Option<PaletteProgramStats> {self.palette_programs.as_ref().map(|m|m.stats())}
    pub fn blend_programs_stats(&self) -> Option<BlendProgramStats> {self.blend_programs.as_ref().map(|m|m.stats())}
    pub fn task_descriptors_stats(&self) -> Option<TaskDescriptorStats> {self.task_descriptors.as_ref().map(|m|m.stats())}
    pub fn effect_event_directories_stats(&self) -> Option<EffectEventDirectoryStats> {self.effect_event_directories.as_ref().map(|m|m.stats())}
    pub fn script_commands_stats(&self) -> Option<ScriptCommandStats> {self.script_commands.as_ref().map(|m|m.stats())}
    pub fn script_archive_stats(&self) -> Option<ScriptArchiveStats> {self.script_archive.as_ref().map(|m|m.stats())}
    pub fn script_text_stats(&self) -> Option<ScriptTextStats> {self.script_text.as_ref().map(|m|m.stats())}
    pub fn script_coordinates_stats(&self) -> Option<ScriptCoordinateStats> {self.script_coordinates.as_ref().map(|m|m.stats())}
    pub fn ui_choice_lists_stats(&self) -> Option<UiChoiceListStats> {self.ui_choice_lists.as_ref().map(|m|m.stats())}
    pub fn unit_obj_initial_pose_stats(&self) -> Option<UnitObjInitialPoseStats> {self.unit_obj_initial_pose.as_ref().map(|m|m.stats())}
    pub fn color_mode_palettes_stats(&self) -> Option<ColorModePaletteStats> {self.color_mode_palettes.as_ref().map(|m|m.stats())}
    pub fn unit_obj_equipment_stats(&self) -> Option<UnitObjEquipmentStats> {self.unit_obj_equipment.as_ref().map(|m|m.stats())}
    pub fn unit_obj_normal_battle_stats(&self) -> Option<UnitObjNormalBattleStats> {self.unit_obj_normal_battle.as_ref().map(|m|m.stats())}
    pub fn unit_obj_alternate_battle_stats(&self) -> Option<UnitObjAlternateBattleStats> {self.unit_obj_alternate_battle.as_ref().map(|m|m.stats())}
    pub fn unit_obj_script_special_normal_stats(&self) -> Option<UnitObjScriptSpecialNormalStats> {self.unit_obj_script_special_normal.as_ref().map(|m|m.stats())}
    pub fn unit_obj_script_special_alternate_stats(&self) -> Option<UnitObjScriptSpecialAlternateStats> {self.unit_obj_script_special_alternate.as_ref().map(|m|m.stats())}
    pub fn unit_obj_script_override_stats(&self) -> Option<UnitObjScriptOverrideStats> {self.unit_obj_script_override.as_ref().map(|m|m.stats())}
    pub fn unit_obj_script_registry_special_stats(&self) -> Option<UnitObjScriptRegistrySpecialStats> {self.unit_obj_script_registry_special.as_ref().map(|m|m.stats())}
    pub fn unit_obj_script_registry_generic_stats(&self) -> Option<UnitObjScriptRegistryGenericStats> {self.unit_obj_script_registry_generic.as_ref().map(|m|m.stats())}
    pub fn unit_obj_property_words_stats(&self) -> Option<UnitObjPropertyWordsStats> {self.unit_obj_property_words.as_ref().map(|m|m.stats())}
    pub fn unit_obj_byte_property_words_stats(&self) -> Option<UnitObjBytePropertyWordsStats> {self.unit_obj_byte_property_words.as_ref().map(|m|m.stats())}
    pub fn unit_obj_ffff_reader_fields_stats(&self) -> Option<UnitObjFfffReaderFieldsStats> {self.unit_obj_ffff_reader_fields.as_ref().map(|m|m.stats())}
    pub fn slot_job_lookup_stats(&self) -> Option<SlotJobLookupStats> {self.slot_job_lookup.as_ref().map(|m|m.stats())}
    pub fn observed_task_parameters_stats(&self) -> Option<ObservedTaskParametersStats> {self.observed_task_parameters.as_ref().map(|m|m.stats())}
    pub fn constructor_obj_animation_stats(&self) -> Option<ConstructorObjAnimationStats> {self.constructor_obj_animation.as_ref().map(|m|m.stats())}
    pub fn task_cycle_deltas_stats(&self) -> Option<TaskCycleDeltasStats> {self.task_cycle_deltas.as_ref().map(|m|m.stats())}
    pub fn ui_raw_transfers_stats(&self)->Option<RawTransfersStats> {self.ui_raw_transfers.as_ref().map(|d|d.stats())}
    pub fn dynamic_raw_fragments_stats(&self)->Option<DynamicFragmentsStats> {self.dynamic_raw_fragments.as_ref().map(|d|d.stats())}
    pub fn indexed_graphics_stats(&self)->Option<IndexedGraphicsStats> {self.indexed_graphics.as_ref().map(|d|d.stats())}
    pub fn world_records38_stats(&self)->Option<WorldRecords38Stats> {self.world_records38.as_ref().map(|d|d.stats())}
    pub fn parameter_records27_stats(&self)->Option<ParameterRecords27Stats> {self.parameter_records27.as_ref().map(|d|d.stats())}
    pub fn event_records12_stats(&self)->Option<EventRecords12Stats> {self.event_records12.as_ref().map(|d|d.stats())}
    pub fn selection_byte_tables_stats(&self)->Option<SelectionByteTablesStats> {self.selection_byte_tables.as_ref().map(|d|d.stats())}
    pub fn mission_records70_stats(&self)->Option<MissionRecords70Stats> {self.mission_records70.as_ref().map(|d|d.stats())}
    pub fn conditional_records8_stats(&self)->Option<ConditionalRecords8Stats> {self.conditional_records8.as_ref().map(|d|d.stats())}
    pub fn records4_stats(&self)->Option<Records4Stats> {self.records4.as_ref().map(|d|d.stats())}
    pub fn metadata28_stats(&self)->Option<Metadata28Stats> {self.metadata28.as_ref().map(|d|d.stats())}
    pub fn encounter_coordinates_stats(&self)->Option<EncounterCoordinatesStats> {self.encounter_coordinates.as_ref().map(|d|d.stats())}
    pub fn encounter_unit_templates_stats(&self) -> Option<EncounterUnitTemplatesStats> {self.encounter_unit_templates.as_ref().map(|m|m.stats())}
    pub fn encounter_headers_stats(&self) -> Option<EncounterHeadersStats> {self.encounter_headers.as_ref().map(|m|m.stats())}
    pub fn world_roads_stats(&self) -> Option<WorldRoadsStats> {self.world_roads.as_ref().map(|m|m.stats())}
    pub fn world_nodes_stats(&self) -> Option<WorldNodesStats> {self.world_nodes.as_ref().map(|m|m.stats())}
    pub fn word_task_obj_stats(&self) -> Option<WordTaskObjStats> {self.word_task_obj_animations.as_ref().map(|m|m.stats())}
    pub fn map_raster_palettes_stats(&self) -> Option<MapRasterPaletteStats> {self.map_raster_palettes.as_ref().map(|m|m.stats())}
    pub fn auxiliary_commands_stats(&self) -> Option<AuxiliaryCommandStats> {self.auxiliary_scripts.as_ref().map(|m|m.command_stats())}
    pub fn auxiliary_archive_stats(&self) -> Option<AuxiliaryArchiveStats> {self.auxiliary_scripts.as_ref().map(|m|m.archive_stats())}
    /// Host audio view: typed owners merge at shared boundaries without guest bus clocks.
    pub fn audio_host_read<const N:usize>(&self,index:usize,rom:&[u8])->Option<[u8;N]>{
        rom.get(index..index.checked_add(N)?)?;
        let prior=self.audio_directory.as_ref().and_then(|d|d.host_read(index,rom,None));
        let prior = match &self.audio_tones{Some(t)=>t.host_read(index,rom,prior),None=>prior};
        let prior = match &self.audio_sequences{Some(t)=>t.host_read(index,rom,prior),None=>prior};
        match &self.audio_driver{Some(t)=>t.host_read(index,rom,prior),None=>prior}
    }
    pub fn item_link_stats(&self) -> Option<ItemLinkStats> {self.item_links.as_ref().map(|l|l.stats())}
    pub fn item_link_file_sha256(&self) -> Option<&str> {self.item_links.as_ref().map(|l|l.file_sha256())}
    pub fn item_stats(&self) -> Option<ItemStats> { self.items.as_ref().map(|i|i.stats()) }
    pub fn item_file_sha256(&self) -> Option<&str> {self.items.as_ref().map(|i|i.file_sha256())}
    pub fn table_graphics_stats(&self) -> Option<TableGraphicsStats> { self.ui_graphics.as_ref().map(|g| g.table_stats()) }
    pub fn ui_graphics_stats(&self) -> Option<UiGraphicsStats> { self.ui_graphics.as_ref().map(|g| g.stats()) }
    pub fn ui_graphics_file_sha256(&self) -> Option<&str> { self.ui_graphics.as_ref().map(|g| g.file_sha256()) }
    pub fn ui_graphics_used_sources(&self) -> Vec<(u32,u64)> { self.ui_graphics.as_ref().map_or_else(Vec::new, |g| g.used_sources()) }
    pub fn ui_palette_stats(&self) -> Option<UiPaletteStats> { self.ui_palettes.as_ref().map(|p| p.stats()) }
    pub fn ui_palette_file_sha256(&self) -> Option<&str> { self.ui_palettes.as_ref().map(|p| p.file_sha256()) }
    pub fn ui_palette_source_bytes(&self, offset: u32) -> Option<u64> { self.ui_palettes.as_ref()?.source_bytes(offset) }
    pub fn obj_graphics_stats(&self) -> Option<ObjGraphicsStats> { self.obj_graphics.as_ref().map(|g| g.stats()) }
    pub fn obj_graphics_file_sha256(&self) -> Option<&str> { self.obj_graphics.as_ref().map(|g| g.file_sha256()) }
    pub fn obj_animation_stats(&self) -> Option<ObjAnimationStats> { self.obj_animations.as_ref().map(|a| a.stats()) }

    pub fn obj_list_stats(&self) -> Option<ObjListStats> { self.obj_lists.as_ref().map(|l| l.stats()) }

    /// Cumulative source coverage (guest bus, host descriptor), outside snapshots.
    pub fn obj_list_source_used(&self, offset: u32) -> Option<(bool, bool)> { self.obj_lists.as_ref()?.source_used(offset) }

    pub fn obj_list_used_sources(&self) -> Vec<(u32, bool, bool)> { self.obj_lists.as_ref().map_or_else(Vec::new, |l| l.used_sources()) }

    pub(crate) fn obj_list(&self, address: u32) -> Option<&super::obj_lists::List> {
        if !matches!(address >> 24, 8 | 9) { return None; }
        self.obj_lists.as_ref().and_then(|l| l.host_list(address & 0x1ffffff))
            .or_else(|| self.obj_animations.as_ref()?.host_list(address & 0x1ffffff))
            .or_else(|| self.word_task_obj_animations.as_ref()?.host_list(address & 0x1ffffff))
    }

    pub fn font_keyboard_stats(&self) -> Option<FontKeyboardStats> { self.font_keyboard.as_ref().map(|f| f.stats()) }

    pub(crate) fn name_keyboard_row_widths(&self, root: usize, page: usize) -> Option<&[u8]> {
        if root != super::name_keyboard::ROW_WIDTH_ROOT as usize { return None; }
        self.font_keyboard.as_ref()?.row_widths(page)
    }

    pub fn stats(&self) -> ResourceStats {
        ResourceStats {
            bus_reads: self.bus_reads.load(Ordering::Relaxed),
            bus_source_mask: self.bus_source_mask.load(Ordering::Relaxed) as u8,
            host_rom_reads: self.host_rom_reads.load(Ordering::Relaxed),
            host_ram_reads: self.host_ram_reads.load(Ordering::Relaxed),
            dynamic_ram_reads: self.dynamic_ram_reads.load(Ordering::Relaxed),
            validation_reads: self.validation_reads.load(Ordering::Relaxed),
        }
    }

    // This is a fixed regional mapping, never a manifest/range catalog search.
    fn source_offset(&self, index: usize, len: usize) -> Option<usize> {
        let overlaps = |base: usize| index < base + 48 && index.saturating_add(len) > base;
        match self.version {
            Version::Usa if overlaps(0x149510) => Some(0x149510),
            Version::Usa if overlaps(0xa398dc) => Some(0xa398dc),
            Version::Europe if overlaps(0x14ced8) => Some(0x14ced8),
            Version::Europe if overlaps(0xd976e8) => Some(0xd976e8),
            _ => None,
        }
    }

    /// Only the ROM read arms call this, after their GPIO/EEPROM and bounds
    /// checks, and only on the fixed metadata source pages. Crossing reads
    /// keep adjacent opaque ROM bytes without reading a migrated byte there.
    pub(crate) fn guest_read<const N: usize>(&self, index: usize, rom: &[u8]) -> Option<[u8; N]> {
        let Some(base) = self.source_offset(index, N) else {
            let prior = self
                .background
                .as_ref()
                .and_then(|background| background.guest_read(index, rom))
                .or_else(|| self.palettes.as_ref().and_then(|palettes| palettes.guest_read(index, rom)))
                .or_else(|| self.alternate_palettes.as_ref().and_then(|palettes| palettes.guest_read(index, rom)));
            // Packet and base-palette spans interleave in the same asset pages.
            // Overlay each mapped packet byte onto the prior-family result.
            let prior = match &self.packet_programs {
                Some(packets) => packets.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.main_graphics {
                Some(graphics) => graphics.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.transfers {
                Some(transfers) => transfers.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.short_texts {
                Some(texts) => texts.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.text_catalogs {
                Some(texts) => texts.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.font_keyboard {
                Some(f) => f.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.obj_lists {
                Some(lists) => lists.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.obj_animations {
                Some(banks) => banks.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.obj_graphics {
                Some(graphics) => graphics.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.ui_palettes {
                Some(palettes) => palettes.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.ui_graphics {
                Some(graphics) => graphics.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.items {
                Some(items) => items.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.item_links {
                Some(links) => links.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.metadata16 {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.metadata52 {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.metadata4 {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.metadata2 {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.default48 {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.constructor_alternates {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.metadata14 {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.metadata12 {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.global_pair_flags {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.evaluation_parameters {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.progression_rewards {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.progression_gates {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.progression_selection_gates {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.aggregate_flag_keys {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.trade_parameters {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.item_effect_parameters {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.audio_directory {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.audio_tones {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.audio_sequences {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.audio_driver {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.palette_effect_dispatch {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.palette_programs {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.blend_programs {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.task_descriptors {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.effect_event_directories {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.script_commands {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.script_archive {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.script_text {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.script_coordinates {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.auxiliary_scripts {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.map_raster_palettes {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.ui_choice_lists {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_initial_pose {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.color_mode_palettes {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_equipment {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_normal_battle {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_alternate_battle {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_script_special_normal {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_script_special_alternate {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_script_override {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_script_registry_special {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_script_registry_generic {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_property_words {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_byte_property_words {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.unit_obj_ffff_reader_fields {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.slot_job_lookup {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.observed_task_parameters {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.constructor_obj_animation {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.task_cycle_deltas {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.word_task_obj_animations {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.world_nodes {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.world_roads {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.encounter_headers {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.encounter_unit_templates {
                Some(data) => data.guest_read(index, rom, prior),
                None => prior,
            };
            let prior = match &self.encounter_coordinates {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.metadata28 {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.records4 {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.conditional_records8 {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.mission_records70 {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.selection_byte_tables {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.event_records12 {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.parameter_records27 {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.world_records38 {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.indexed_graphics {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.ui_raw_transfers {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.dynamic_raw_fragments {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.conditional_four_byte_lookup {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.conditional_packed_lookup {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.conditional_descriptor_banks {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.conditional_descriptor_bank_zero {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.conditional_property_masks {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.compact_obj_animations {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.secondary_compact_obj_animations {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.tertiary_compact_obj_animations {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.quaternary_compact_obj_animations {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            let prior = match &self.quinary_compact_obj_animations {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
            return match &self.senary_compact_obj_animations {Some(data)=>data.guest_read(index,rom,prior),None=>prior};
        };
        self.bus_reads.fetch_add(1, Ordering::Relaxed);
        let source_bit = if matches!(base, 0x149510 | 0x14ced8) {
            1
        } else {
            2
        };
        self.bus_source_mask.fetch_or(source_bit, Ordering::Relaxed);
        Some(std::array::from_fn(|i| {
            match index
                .checked_add(i)
                .and_then(|at| at.checked_sub(base))
                .and_then(|at| self.projection.get(at))
            {
                Some(&byte) => byte,
                None => rom[index + i],
            }
        }))
    }

    pub(crate) fn source_table(&self, offset: usize) -> Option<&[u8]> {
        if self.source_offset(offset, 48) != Some(offset) {
            return None;
        }
        self.validation_reads.fetch_add(1, Ordering::Relaxed);
        Some(&self.projection)
    }

    pub(crate) fn host_dimensions(
        &self,
        address: u32,
        ram_record: Option<&[u8]>,
    ) -> Option<(u16, u16)> {
        let offset = match address >> 24 {
            0x08..=0x0d => {
                let offset = (address & 0x1ffffff) as usize;
                let base = self.source_offset(offset, 4)?;
                let relative = offset.checked_sub(base)?;
                if relative % 4 != 0 || relative >= 48 {
                    return None;
                }
                self.host_rom_reads.fetch_add(1, Ordering::Relaxed);
                relative
            }
            0x03 if (0x03006d28..0x03006d58).contains(&address) && address & 3 == 0 => {
                let offset = (address - 0x03006d28) as usize;
                if ram_record != Some(&self.projection[offset..offset + 4]) {
                    self.dynamic_ram_reads.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
                self.host_ram_reads.fetch_add(1, Ordering::Relaxed);
                offset
            }
            _ => return None,
        };
        let size = &self.dimensions.entries[offset / 4];
        Some((size.width, size.height))
    }

    #[cfg(test)]
    pub(crate) fn synthetic_obj_lists(rom: &[u8], dimensions: ObjDimensions, lists: super::obj_lists::Lists) -> Arc<Self> {
        let mut resources = Self::new(ffta_rom::digest(rom), "synthetic-lists".into(), "obj".into(), Version::Europe, dimensions);
        resources.obj_lists = Some(Box::new(ObjListResources::new(lists).unwrap()));
        Arc::new(resources)
    }

    #[cfg(test)]
    pub(crate) fn synthetic(rom: &[u8], version: Version, dimensions: ObjDimensions) -> Arc<Self> {
        Arc::new(Self::new(
            ffta_rom::digest(rom),
            "synthetic-dataset".into(),
            "synthetic-file".into(),
            version,
            dimensions,
        ))
    }
}

impl crate::MemMap {
    pub(crate) fn name_keyboard_row_widths(&self, root: usize, page: usize) -> Option<&[u8]> {
        if let Some(rows) = self.rom_resources.as_ref()
            .and_then(|resources| resources.name_keyboard_row_widths(root, page)) {
            #[cfg(feature = "rom-data-read-audit")]
            super::read_audit::record_served(super::read_audit::Reader::KeyboardWidths, 0x08000000 + (root + page * 6) as u32, 6, true);
            return Some(rows);
        }
        let start = root.checked_add(page.checked_mul(6)?)?;
        let rows = self.rom.get(start..start.checked_add(6)?)?;
        #[cfg(feature = "rom-data-read-audit")]
        super::read_audit::record_served(super::read_audit::Reader::KeyboardWidths, 0x08000000 + start as u32, 6, false);
        Some(rows)
    }
    /// Transactional source binding. Base resources preserve the original image
    /// identity and guest bytes, so snapshots/native translations remain keyed
    /// to the original ROM. Modified resources require a different future API.
    pub fn set_rom_resources(
        &mut self,
        resources: Option<Arc<RuntimeResources>>,
    ) -> Result<(), String> {
        if let Some(data) = &resources {
            if data.source_sha256() != ffta_rom::digest(&self.rom) {
                return Err("Decoded game data belong to a different ROM".into());
            }
        }
        self.rom_resources = resources;
        Ok(())
    }

    pub fn rom_resources(&self) -> Option<&Arc<RuntimeResources>> {
        self.rom_resources.as_ref()
    }

    pub(crate) fn obj_dimensions(&self, address: u32) -> Option<(u16, u16)> {
        // Preserve illegal shape-3/out-of-table and genuinely changed guest RAM.
        let ram = if address >> 24 == 3 {
            let offset = (address & 0x7fff) as usize;
            self.iwram.get(offset..offset + 4)
        } else {
            None
        };
        self.rom_resources.as_ref()?.host_dimensions(address, ram)
    }

    pub(crate) fn obj_source_table(&self, offset: usize) -> Option<&[u8]> {
        if let Some(resources) = &self.rom_resources {
            if let Some(bytes) = resources.source_table(offset) {
                #[cfg(feature = "rom-data-read-audit")]
                super::read_audit::record_served(super::read_audit::Reader::ObjSourceTable, 0x08000000 + offset as u32, 48, true);
                return Some(bytes);
            }
        }
        let bytes = self.rom.get(offset..offset.checked_add(48)?)?;
        #[cfg(feature = "rom-data-read-audit")]
        super::read_audit::record_served(super::read_audit::Reader::ObjSourceTable, 0x08000000 + offset as u32, 48, false);
        Some(bytes)
    }

    #[inline]
    pub(crate) fn resource_page(index: usize) -> bool {
        matches!(index >> 12, 0x149 | 0x14c | 0xa39 | 0xd97
            | 0x569..=0x56c | 0x766..=0x769)
            || PaletteResources::source_page(index)
            || AlternatePaletteResources::source_page(index)
            || PacketResources::source_page(index)
            || GraphicsResources::source_page(index)
            || TransferResources::source_page(index)
            || TextResources::source_page(index)
            || TextCatalogResources::source_page(index)
            || FontKeyboardResources::source_page(index)
            || ObjListResources::source_page(index)
            || ObjAnimationResources::source_page(index)
            || WordTaskObjResources::source_page(index)
            || WorldRoadsResources::source_page(index)
            || EncounterHeadersResources::source_page(index)
            || EncounterUnitTemplatesResources::source_page(index)
            || EncounterCoordinatesResources::source_page(index)
            || Metadata28Resources::source_page(index)
            || Records4Resources::source_page(index)
            || ConditionalRecords8Resources::source_page(index)
            || MissionRecords70Resources::source_page(index)
            || SelectionByteTablesResources::source_page(index)
            || EventRecords12Resources::source_page(index)
            || ParameterRecords27Resources::source_page(index)
            || WorldRecords38Resources::source_page(index)
            || IndexedGraphicsResources::source_page(index)
            || RawTransfersResources::source_page(index)
            || ui_raw_transfers::DynamicFragmentsResources::source_page(index)
            || ConditionalLookupResources::source_page(index)
            || PackedLookupResources::source_page(index)
            || DescriptorBanksResources::source_page(index)
            || DescriptorBankZeroResources::source_page(index)
            || PropertyMasksResources::source_page(index)
            || CompactObjResources::source_page(index)
            || SecondaryCompactObjResources::source_page(index)
            || TertiaryCompactObjResources::source_page(index)
            || QuaternaryCompactObjResources::source_page(index)
            || QuinaryCompactObjResources::source_page(index)
            || SenaryCompactObjResources::source_page(index)
            || ObjGraphicsResources::source_page(index)
            || UiPaletteResources::source_page(index)
            || UiGraphicsResources::source_page(index)
            || ItemResources::source_page(index)
            || ItemLinkResources::source_page(index)
            || Metadata16Resources::source_page(index)
            || Metadata52Resources::source_page(index)
            || Metadata4Resources::source_page(index)
            || Metadata2Resources::source_page(index)
            || Default48Resources::source_page(index)
            || ConstructorAlternatesResources::source_page(index)
            || Metadata14Resources::source_page(index)
            || Metadata12Resources::source_page(index)
            || GlobalPairFlagsResources::source_page(index)
            || EvaluationParametersResources::source_page(index)
            || ProgressionRewardsResources::source_page(index)
            || ProgressionGatesResources::source_page(index)
            || ProgressionSelectionGatesResources::source_page(index)
            || AggregateFlagKeysResources::source_page(index)
            || TradeParametersResources::source_page(index)
            || ItemEffectParametersResources::source_page(index)
            || AudioDirectoryResources::source_page(index)
            || AudioTonesResources::source_page(index)
            || AudioSequencesResources::source_page(index)
            || AudioDriverResources::source_page(index)
            || PaletteEffectDispatchResources::source_page(index)
            || PaletteProgramResources::source_page(index)
            || BlendProgramResources::source_page(index)
            || TaskDescriptorResources::source_page(index)
            || EffectEventDirectoryResources::source_page(index)
            || ScriptCommandResources::source_page(index)
            || ScriptArchiveResources::source_page(index)
            || ScriptTextResources::source_page(index)
            || ScriptCoordinateResources::source_page(index)
            || UiChoiceListResources::source_page(index)
            || UnitObjInitialPoseResources::source_page(index)
            || ColorModePaletteResources::source_page(index)
            || UnitObjEquipmentResources::source_page(index)
            || UnitObjNormalBattleResources::source_page(index)
            || UnitObjAlternateBattleResources::source_page(index)
            || UnitObjScriptSpecialNormalResources::source_page(index)
            || UnitObjScriptSpecialAlternateResources::source_page(index)
            || UnitObjScriptOverrideResources::source_page(index)
            || UnitObjScriptRegistrySpecialResources::source_page(index)
            || UnitObjScriptRegistryGenericResources::source_page(index)
            || UnitObjPropertyWordsResources::source_page(index)
            || UnitObjBytePropertyWordsResources::source_page(index)
            || UnitObjFfffReaderFieldsResources::source_page(index)
            || SlotJobLookupResources::source_page(index)
            || ObservedTaskParametersResources::source_page(index)
            || MapRasterPaletteResources::source_page(index)
            || AuxiliaryScriptResources::source_page(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bus, Machine, MemMap};

    pub(crate) fn dimensions() -> ObjDimensions {
        ObjDimensions {
            entries: std::array::from_fn(|i| ObjSize {
                width: 8 << (i % 4),
                height: 8 << ((i + 1) % 4),
            }),
        }
    }

    fn machine(version: Version) -> (Machine, Arc<RuntimeResources>, Vec<usize>) {
        let dimensions = dimensions();
        let bases = match version {
            Version::Usa => vec![0x149510, 0xa398dc],
            Version::Europe => vec![0x14ced8, 0xd976e8],
        };
        let mut rom = vec![0x53; 0x1000000];
        for &base in &bases {
            rom[base..base + 48].copy_from_slice(&dimensions.source_bytes());
        }
        let resources = RuntimeResources::synthetic(&rom, version, dimensions);
        let mut machine = Machine::new(rom);
        machine
            .bus
            .set_rom_resources(Some(resources.clone()))
            .unwrap();
        (machine, resources, bases)
    }

    #[test]
    fn every_bus_width_mirror_boundary_uses_decoded_data_and_preserves_neighbors() {
        for version in [Version::Usa, Version::Europe] {
            let (mut machine, resources, bases) = machine(version);
            let original = machine.bus.rom.clone();
            for &base in &bases {
                machine.bus.rom[base..base + 48].fill(0xcd);
            }
            let before = machine.bus.clock;
            for &base in &bases {
                for mirror in [0x08000000, 0x0a000000, 0x0c000000] {
                    for offset in base - 4..base + 52 {
                        let address = mirror + offset as u32;
                        assert_eq!(machine.bus.read8(address), original[offset]);
                        assert_eq!(
                            machine.bus.read16(address),
                            u16::from_le_bytes(original[offset..offset + 2].try_into().unwrap())
                        );
                        assert_eq!(
                            machine.bus.read32(address),
                            u32::from_le_bytes(original[offset..offset + 4].try_into().unwrap())
                        );
                    }
                }
            }
            assert!(resources.stats().bus_reads > 0);
            assert_eq!(resources.stats().bus_source_mask, 3);
            assert_eq!(machine.bus.clock, before);
            assert_eq!(machine.bus.read8(0x09000000), 0);
        }
    }

    #[test]
    fn original_cpu_instructions_and_dma_execute_with_unchanged_timing() {
        let (mut machine, resources, bases) = machine(Version::Usa);
        let rom = machine.bus.rom.clone();
        let mut reference = Machine::new(rom);
        // Original own-code LDR r0,[r1]; LDRH r2,[r1]; LDRB r3,[r1].
        for m in [&mut reference, &mut machine] {
            m.cpu.regs[15] = 0x02000000;
            m.cpu.regs[1] = 0x08149510;
            for (i, word) in [0xe5910000u32, 0xe1d120b0, 0xe5d13000]
                .into_iter()
                .enumerate()
            {
                m.bus.write32(0x02000000 + i as u32 * 4, word);
            }
        }
        for &base in &bases {
            machine.bus.rom[base..base + 48].fill(0xcd);
        }
        let pointer = &mut machine as *mut Machine as *mut std::ffi::c_void;
        assert_eq!(
            (crate::capi::RT_API.read8)(pointer, 0x08149510),
            resources.projection[0] as u32
        );
        assert_eq!(
            (crate::capi::RT_API.read16)(pointer, 0x08149510),
            u16::from_le_bytes(resources.projection[..2].try_into().unwrap()) as u32
        );
        assert_eq!(
            (crate::capi::RT_API.read32)(pointer, 0x08149510),
            u32::from_le_bytes(resources.projection[..4].try_into().unwrap())
        );
        for _ in 0..3 {
            reference.step();
            machine.step();
            assert_eq!(machine.cpu.regs, reference.cpu.regs);
            assert_eq!(machine.cpu.cpsr, reference.cpu.cpsr);
            assert_eq!(machine.bus.clock, reference.bus.clock);
        }
        for words in [false, true] {
            for m in [&mut reference, &mut machine] {
                m.bus.write32(0x040000d4, 0x08149510);
                m.bus.write32(0x040000d8, 0x03006d28);
                m.bus.write16(0x040000dc, if words { 12 } else { 24 });
                m.bus
                    .write16(0x040000de, if words { 0x8400 } else { 0x8000 });
            }
            assert_eq!(machine.bus.iwram, reference.bus.iwram);
            assert_eq!(machine.bus.clock, reference.bus.clock);
            assert_eq!(&machine.bus.iwram[0x6d28..0x6d58], &resources.projection);
        }
        assert!(resources.stats().bus_reads >= 39);
    }

    #[test]
    fn host_typed_and_ram_reads_preserve_dynamic_records_and_invalid_shapes() {
        let (mut machine, resources, bases) = machine(Version::Usa);
        for &base in &bases {
            machine.bus.rom[base..base + 48].fill(0xcd);
        }
        machine.bus.iwram[0x6d28..0x6d58].copy_from_slice(&resources.projection);
        for i in 0..12 {
            let expected = &resources.dimensions.entries[i];
            assert_eq!(
                machine.bus.obj_dimensions(0x08149510 + i as u32 * 4),
                Some((expected.width, expected.height))
            );
            assert_eq!(
                machine.bus.obj_dimensions(0x03006d28 + i as u32 * 4),
                Some((expected.width, expected.height))
            );
        }
        machine.bus.iwram[0x6d28..0x6d2a].copy_from_slice(&123u16.to_le_bytes());
        assert_eq!(machine.bus.obj_dimensions(0x03006d28), None);
        assert_eq!(machine.bus.obj_dimensions(0x03006d58), None);
        assert_eq!(machine.bus.obj_dimensions(0x08149540), None);
        assert_eq!(
            machine.bus.obj_source_table(0x149510),
            Some(resources.projection.as_slice())
        );
        assert_eq!(
            resources.stats(),
            ResourceStats {
                host_rom_reads: 12,
                host_ram_reads: 12,
                dynamic_ram_reads: 1,
                validation_reads: 1,
                ..ResourceStats::default()
            }
        );
    }

    #[test]
    fn snapshot_bytes_and_transactional_load_preserve_nonserialized_resources() {
        let (mut machine, resources, _) = machine(Version::Europe);
        let mut reference = Machine::new(machine.bus.rom.clone());
        let state = reference.save_state().unwrap();
        assert_eq!(state, machine.save_state().unwrap());
        machine.bus.read32(0x0814ced8);
        machine.bus.iwram[5] = 27;
        let count = resources.stats();
        let mut damaged = state.clone();
        *damaged.last_mut().unwrap() ^= 1;
        assert!(machine.load_state(&damaged).is_err());
        assert!(Arc::ptr_eq(
            machine.bus.rom_resources().unwrap(),
            &resources
        ));
        assert_eq!(machine.bus.iwram[5], 27);
        machine.load_state(&state).unwrap();
        assert!(Arc::ptr_eq(
            machine.bus.rom_resources().unwrap(),
            &resources
        ));
        assert_eq!(resources.stats(), count);
        assert_eq!(machine.save_state().unwrap(), state);
        reference
            .load_state(&machine.save_state().unwrap())
            .unwrap();
        let other = MemMap::new(vec![1; 256]);
        let mut incompatible = other;
        assert!(incompatible.set_rom_resources(Some(resources)).is_err());
        assert!(incompatible.rom_resources().is_none());
    }
}
