use std::{
    collections::HashMap,
    ffi::c_void,
    ptr::null,
    sync::{LazyLock, Mutex, OnceLock},
};

use crate::{
    kreide::types::{
        RPG_Client_AvatarHelper, RPG_Client_CachedAssetLoader, RPG_Client_UIGameEntityUtils, RPG_GameCore_AttackType__Boxed, RPG_GameCore_AvatarExcelTable, RPG_GameCore_AvatarPropertyExcelTable, RPG_GameCore_AvatarPropertyType, RPG_GameCore_AvatarRow, RPG_GameCore_MonsterDataComponent, RPG_GameCore_MonsterTemplateExcelTable, RPG_GameCore_ServantDataComponent, UnityEngine_Graphics, UnityEngine_ImageConversion, UnityEngine_Rect, UnityEngine_RenderTexture, UnityEngine_Sprite, UnityEngine_Texture2D
    },
    models::types::{Avatar, Skill},
};
use anyhow::{Context, Result, anyhow};
use function_name::named;
use il2cpp_runtime::{
    Il2CppClass, Il2CppObject, System_RuntimeType, get_cached_class,
    types::{Il2CppString, System_Enum, System_Type},
};

use super::types::{
    RPG_Client_TextID, RPG_Client_TextmapStatic,
    RPG_GameCore_BattleInstance, RPG_GameCore_GameEntity,
    RPG_GameCore_SkillData,
};

fn sanitize_entity_name<S: AsRef<str>>(name: S) -> String {
    let name = name.as_ref();
    if !name.contains("<ub>") && !name.contains("</ub>") {
        return name.to_string();
    }

    name.replace("<ub>", "").replace("</ub>", "")
}

pub fn get_textmap_content(hash: &RPG_Client_TextID) -> Result<String> {
    Ok(unsafe { RPG_Client_TextmapStatic::get_text(hash, null()) }.map(|s| s.to_string())?)
}


#[named]
pub fn get_avatar_data_from_id(avatar_id: u32) -> Result<RPG_GameCore_AvatarRow> {
    log::debug!(function_name!());
    Ok(unsafe { RPG_GameCore_AvatarExcelTable::GetData(avatar_id)? })
}

#[named]
pub unsafe fn get_avatar_from_id(avatar_id: u32) -> Result<Avatar> {
    log::debug!(function_name!());

    let avatar_data = get_avatar_data_from_id(avatar_id)
        .context(format!("AvatarData with id {avatar_id} was null"))?;

    let avatar_name = unsafe { RPG_Client_AvatarHelper::GetAvatarName(avatar_id)?.to_string() };

    Ok(Avatar {
        id: avatar_id,
        name: sanitize_entity_name(avatar_name),
    })
}

#[named]
pub unsafe fn get_skill_from_skilldata(skill_data: RPG_GameCore_SkillData) -> Result<Skill> {
    log::debug!(function_name!());

    if skill_data.0.is_null() {
        return Err(anyhow!("SkillData was null"));
    }

    let row_data = skill_data.RowData()?;

    let text_id = unsafe { row_data.get_SkillName()? };

    let skill_type = unsafe {
        let boxed = RPG_GameCore_AttackType__Boxed(System_Enum::to_object_from_int(
            get_type_handle("RPG.GameCore.AttackType")?,
            row_data.get_AttackType()? as i32,
        )?);
        System_Enum::get_name(get_type_handle("RPG.GameCore.AttackType")?, boxed.0)?.to_string()
    };

    Ok(Skill {
        name: get_textmap_content(&text_id)?,
        skill_type,
        skill_config_id: isize::try_from(*skill_data.SkillConfigID()?)?,
    })
}

#[named]
pub unsafe fn get_avatar_from_entity(entity: RPG_GameCore_GameEntity) -> Result<Avatar> {
    log::debug!(function_name!());

    if entity.0.is_null() {
        return Err(anyhow!("Avatar entity was null"));
    }

    let id = unsafe { RPG_Client_UIGameEntityUtils::get_avatar_id(entity) }
        .context("Failed to get AvatarID from GameEntity")?;

    let avatar_data =
        get_avatar_data_from_id(id).context(format!("AvatarData with id {id} was null"))?;

    let name = unsafe {
        RPG_Client_AvatarHelper::GetAvatarName(id)?.to_string()
    };

    Ok(Avatar {
        id,
        name: sanitize_entity_name(name),
    })
}

#[named]
pub unsafe fn get_avatar_from_servant_entity(entity: RPG_GameCore_GameEntity) -> Result<Avatar> {
    log::debug!(function_name!());

    if entity.0.is_null() {
        return Err(anyhow!("Servant Entity was null"));
    }

    let battle_instance = entity
        ._OwnerWorldRef()?
        ._BattleInstanceRef_k__BackingField()?;

    let entity_manager = battle_instance._GameWorld()?._EntityManager()?;
    let avatar_entity = unsafe { entity_manager.get_entity_summoner(entity)? };
    unsafe { get_avatar_from_entity(avatar_entity) }
}

#[named]
pub unsafe fn get_monster_from_entity(entity: RPG_GameCore_GameEntity) -> Result<Avatar> {
    log::debug!(function_name!());
    let monster_data_comp = RPG_GameCore_MonsterDataComponent(
        unsafe {
            entity.get_component(System_RuntimeType::from_name(
                "RPG.GameCore.MonsterDataComponent",
            )?)?
        }
        .0,
    );

    if monster_data_comp.0.is_null() {
        return Err(anyhow!("entity does not have MonsterDataComponent!"));
    }

    let monster_name = monster_data_comp._MonsterRowData()?._Row()?.MonsterName()?;

    let monster_id = unsafe { monster_data_comp.get_monster_id()? };

    Ok(Avatar {
        id: monster_id,
        name: sanitize_entity_name(get_textmap_content(&*monster_name)?),
    })
}

#[named]
pub unsafe fn get_servant_from_entity(entity: RPG_GameCore_GameEntity) -> Result<Avatar> {
    log::debug!(function_name!());
    let servant_data_comp = RPG_GameCore_ServantDataComponent(
        unsafe {
            entity.get_component(System_RuntimeType::from_name(
                "RPG.GameCore.ServantDataComponent",
            )?)?
        }
        .0,
    );

    if servant_data_comp.0.is_null() {
        return Err(anyhow!("entity does not have ServantDataComponent!"));
    }

    let servant_row = servant_data_comp._ServantRowData()?._Row()?;

    Ok(Avatar {
        id: u32::try_from(*servant_row.ServantID()?)?,
        name: sanitize_entity_name(get_textmap_content(&*servant_row.ServantName()?)?),
    })
}

// #[named]
// pub unsafe fn get_entity_modifiers(entity: RPG_GameCore_GameEntity) -> Result<Vec<Value>> {
//     log::debug!(function_name!());
//     let ability_comp = RPG_GameCore_AbilityComponent(
//         entity
//             .get_component(System_RuntimeType::from_name("RPG.GameCore.AbilityComponent")?)?
//             .0,
//     );

//     if ability_comp.0.is_null() {
//         return Err(anyhow!("entity does not have AbilityComponent!"));
//     }

//     let modifier_list = List(ability_comp._ModifierList()?.0);
//     let modifier_list_array = modifier_list.to_vec::<RPG_GameCore_TurnBasedModifierInstance>();

//     Ok(modifier_list_array
//         .iter()
//         .filter_map(|obj| {
//             let status_config_key = obj.get_key_for_status_config().ok()?;

//             let status_row =
//                 RPG_GameCore_StatusExcelTable::get_by_modifier_name(status_config_key).ok()?;

//             Some(if status_row.is_null() {
//                 json!({
//                     "key": status_config_key.as_str(),
//                 })
//             } else {
//                 json!({
//                     "key": status_config_key.as_str(),
//                     "desc": get_textmap_content(&status_row.StatusDesc().ok()?),
//                     "name": get_textmap_content(&status_row.StatusName().ok()?),
//                 })
//             })
//         })
//         .collect::<Vec<_>>())
// }

// pub unsafe fn get_entity_ability_properties(
//     entity: RPG_GameCore_GameEntity,
// ) -> Result<HashMap<String, f64>> {
//     let ability_comp = RPG_GameCore_TurnBasedAbilityComponent(
//         unsafe {
//             entity.get_component(System_RuntimeType::from_name(
//                 "RPG.GameCore.TurnBasedAbilityComponent",
//             )?)?
//         }
//         .0,
//     );

//     if ability_comp.0.is_null() {
//         return Err(anyhow!("entity does not have TurnBasedAbilityComponent!"));
//     }

//     Ok((0..=193)
//         .filter_map(|i| {
//             let property_enum =
//                 unsafe { std::mem::transmute::<i32, RPG_GameCore_AbilityProperty>(i) };
//             let value = fixpoint_to_raw(&unsafe { ability_comp.get_property(property_enum).ok()? });

//             (value != 0.0).then_some((format!("{property_enum:?}"), value))
//         })
//         .collect::<HashMap<String, f64>>())
// }

#[named]
pub unsafe fn get_monster_from_runtime_id(
    id: u32,
    battle_instance: RPG_GameCore_BattleInstance,
) -> Result<Avatar> {
    log::debug!(function_name!());
    unsafe {
        get_monster_from_entity(
            battle_instance
                ._GameWorld()?
                ._EntityManager()?
                .get_entity_by_runtime_id(id)?,
        )
    }
}

pub fn is_obfuscated_name<S: AsRef<str>>(name: S) -> bool {
    let name = name.as_ref();
    name.len() == 11 && name.chars().all(|c| c.is_ascii_uppercase())
}

pub fn get_type_handle<S: AsRef<str>>(type_name: S) -> Result<System_Type> {
    let type_name = type_name.as_ref();
    let runtime_type = System_RuntimeType::from_name(type_name)?;
    let ty = runtime_type.get_il2cpp_type();
    Ok(unsafe { System_Type::get_type_from_handle(ty)? })
}

unsafe extern "C" {
    // src/guard.c
    fn veritas_guarded_call2(
        method: *const c_void,
        arg0: *const c_void,
        arg1: *const c_void,
        result: *mut *const c_void,
        code: *mut u32,
        il2cpp_exception: *mut *const c_void,
    ) -> i32;
}

/// "ExceptionType: message" for a managed Il2CppException*.
unsafe fn describe_il2cpp_exception(exception: *const c_void) -> String {
    // Il2CppException (Unity 2019.4, .NET 4.x): Il2CppObject header, className, message.
    let describe = || unsafe {
        let class = Il2CppClass(*(exception as *const *const c_void));
        let message = *(exception.byte_add(0x18) as *const *const c_void);
        let message = if message.is_null() {
            String::new()
        } else {
            Il2CppString(message).to_string()
        };
        format!("{}: {}", class.qualified_name(), message)
    };
    microseh::try_seh(describe).unwrap_or_else(|e| format!("<unreadable exception object: {e}>"))
}

/// Calls a two-argument static IL2CPP method through src/guard.c, so a managed exception
/// comes back as an error. Uncaught, it would unwind into Rust and abort the whole game.
unsafe fn guarded_call2(
    method: usize,
    arg0: *const c_void,
    arg1: *const c_void,
) -> Result<*const c_void> {
    let mut result = null();
    let mut code = 0u32;
    let mut exception = null();
    let status = unsafe {
        veritas_guarded_call2(
            method as *const c_void,
            arg0,
            arg1,
            &mut result,
            &mut code,
            &mut exception,
        )
    };
    if status == 0 {
        return Ok(result);
    }
    Err(anyhow!(
        "threw {}",
        if exception.is_null() {
            format!("exception code {code:#X}")
        } else {
            unsafe { describe_il2cpp_exception(exception) }
        }
    ))
}

fn enum_method(cache: &OnceLock<usize>, name: &str, arg_types: Vec<&str>) -> Result<usize> {
    if let Some(va) = cache.get() {
        return Ok(*va);
    }
    let method = get_cached_class("System.Enum")?.find_method(name, arg_types)?;
    Ok(*cache.get_or_init(|| method.va() as usize))
}

// Highest underlying value probed when building an enum's name table.
const ENUM_PROBE_LIMIT: i32 = 4096;

/// Value of the named member of an IL2CPP enum.
///
/// On game 4.5.0, `System.Enum.Parse` rejects every string Veritas creates ("Must specify
/// valid information for parsing in the string"), so this instead builds a name → value
/// table per enum with `Enum.ToObject` + `Enum.GetName`, which only pass numbers in.
pub unsafe fn enum_value(type_name: &str, member: &str) -> Result<i32> {
    static TABLES: LazyLock<Mutex<HashMap<String, HashMap<String, i32>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    static TO_OBJECT: OnceLock<usize> = OnceLock::new();
    static GET_NAME: OnceLock<usize> = OnceLock::new();

    let mut tables = TABLES.lock().unwrap_or_else(|e| e.into_inner());
    if !tables.contains_key(type_name) {
        let to_object = enum_method(&TO_OBJECT, "ToObject", vec!["System.Type", "int"])?;
        let get_name = enum_method(&GET_NAME, "GetName", vec!["System.Type", "object"])?;
        let ty = get_type_handle(type_name)?;

        let mut table = HashMap::new();
        for value in 0..=ENUM_PROBE_LIMIT {
            let boxed = unsafe { guarded_call2(to_object, ty.0, value as usize as *const c_void) }
                .with_context(|| format!("System.Enum.ToObject({type_name}, {value})"))?;
            let name = unsafe { guarded_call2(get_name, ty.0, boxed) }
                .with_context(|| format!("System.Enum.GetName({type_name}, {value})"))?;
            if !name.is_null() {
                table.insert(Il2CppString(name).to_string(), value);
            }
        }
        log::debug!("Built {type_name} value table: {} members", table.len());
        tables.insert(type_name.to_string(), table);
    }

    tables[type_name]
        .get(member)
        .copied()
        .ok_or_else(|| anyhow!("{type_name} has no member named {member}"))
}

// UnityEngine.RenderTextureFormat.Default / UnityEngine.RenderTextureReadWrite.Linear.
// Fixed Unity API values; parsing them at runtime throws on game 4.5.0.
const RENDER_TEXTURE_FORMAT_DEFAULT: i32 = 7;
const RENDER_TEXTURE_READ_WRITE_LINEAR: i32 = 1;

/// Common texture rendering pipeline: texture → render target → readable texture → PNG bytes
unsafe fn render_texture_to_png_bytes(tex: UnityEngine_Texture2D) -> Result<Vec<u8>> {
    unsafe {
        let (default_format, rw_format) =
            (RENDER_TEXTURE_FORMAT_DEFAULT, RENDER_TEXTURE_READ_WRITE_LINEAR);

        let render_tex = UnityEngine_RenderTexture::GetTemporary(
            tex.as_base().get_width()?,
            tex.as_base().get_height()?,
            0,
            default_format,
            rw_format,
        )?;
        UnityEngine_Graphics::Blit(tex, render_tex)?;
        let prev = UnityEngine_RenderTexture::GetActive()?;
        UnityEngine_RenderTexture::set_active(render_tex)?;

        use il2cpp_runtime::api::il2cpp_object_new;
        let readable_tex = UnityEngine_Texture2D(il2cpp_object_new(get_cached_class(
            UnityEngine_Texture2D::ffi_name(),
        )?));

        readable_tex.new(tex.as_base().get_width()?, tex.as_base().get_height()?)?;
        readable_tex.read_pixels(
            UnityEngine_Rect {
                x: 0.,
                y: 0.,
                width: render_tex.get_width()? as f32,
                height: render_tex.get_height()? as f32,
            },
            0,
            0,
        )?;
        readable_tex.apply()?;
        UnityEngine_RenderTexture::set_active(prev)?;
        UnityEngine_RenderTexture::ReleaseTemporary(render_tex)?;

        let array = UnityEngine_ImageConversion::EncodeToPNG(readable_tex)?;
        Ok(array.to_vec::<u8>())
    }
}

pub fn get_monster_png_bytes(monster_id: u32) -> Result<Vec<u8>> {
    unsafe {
        let monster_row = RPG_GameCore_MonsterTemplateExcelTable::GetData(monster_id)?;
        let type_handle = get_type_handle(UnityEngine_Sprite::ffi_name())?;

        let sprite = RPG_Client_CachedAssetLoader::SyncLoadAsset(
            monster_row.RoundIconPath()?,
            type_handle,
            false,
        )?;
        let sprite = UnityEngine_Sprite(sprite.0);
        let tex = sprite.get_texture()?;

        render_texture_to_png_bytes(tex)
    }
}

pub fn get_avatar_png_bytes(avatar_id: u32) -> Result<Vec<u8>> {
    unsafe {
        let avatar_row = RPG_GameCore_AvatarExcelTable::GetData(avatar_id)?;
        log::info!(
            "Support Avatar: {}, Icon Path: {}",
            avatar_id,
            avatar_row.AvatarSideIconPath()?.to_string()
        );

        let type_handle = get_type_handle(UnityEngine_Sprite::ffi_name())?;

        let sprite = RPG_Client_CachedAssetLoader::SyncLoadAsset(
            avatar_row.AvatarSideIconPath()?,
            type_handle,
            false,
        )?;
        let sprite = UnityEngine_Sprite(sprite.0);
        let tex = sprite.get_texture()?;

        render_texture_to_png_bytes(tex)
    }
}

pub fn get_property_icon_png_bytes(property_name: &str) -> Result<Vec<u8>> {
    unsafe {
        let property_type: RPG_GameCore_AvatarPropertyType = std::mem::transmute(enum_value(
            "RPG.GameCore.AvatarPropertyType",
            property_name,
        )?);

        let row = RPG_GameCore_AvatarPropertyExcelTable::GetData(property_type)?;
        let icon_path = row.IconPath()?;

        let type_handle = get_type_handle(UnityEngine_Sprite::ffi_name())?;
        
        let sprite = RPG_Client_CachedAssetLoader::SyncLoadAsset(
            icon_path,
            type_handle,
            false,
        )?;
        let sprite = UnityEngine_Sprite(sprite.0);
        let tex = sprite.get_texture()?;

        render_texture_to_png_bytes(tex)
    }
}

pub fn dump_avatar_png_bytes(avatar_id: u32, png_bytes: &[u8]) -> Result<()> {
    use std::fs;
    let out_dir = std::env::current_dir()?.join("avatar_png_dumps");
    fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("{}.png", avatar_id));
    fs::write(&out_path, png_bytes)?;

    log::info!("Saved avatar PNG dump: {}", out_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guarded_call_catches_exception() {
        // Calling address 1 faults; the guard must report it instead of crashing.
        let (mut result, mut code, mut exception) = (null(), 0u32, null());
        let status = unsafe {
            veritas_guarded_call2(1 as *const c_void, null(), null(), &mut result, &mut code, &mut exception)
        };
        assert_eq!(status, 1);
        assert_eq!(code, 0xC0000005);
        assert!(exception.is_null());
    }
}
