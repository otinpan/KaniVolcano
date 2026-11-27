# Entity API

Entity API creates and removes entities and manages components, names, tags, and queries.

It is available from `SceneContext`, `UpdateContext`, and `CommandContext`.

```rust
let entity = context.spawn();
context.add_component(entity, Transform::default())?;
```

An entity is only an ID. It gains behavior and state through components.

## Creating and Removing Entities

```rust
let entity: EntityId = context.spawn();
```

Creates a new entity.

When called from `SceneContext`, `spawn()` automatically adds `SceneOwned`.

```rust
let removed: bool = context.despawn(entity);
```

Despawns an entity and its descendants linked by `ParentComponent`, removing their components and releasing mesh, texture, and glTF model references. Assets with `auto_release=true` are released when their last reference is removed; GPU destruction is queued.

```rust
context.despawn_last();
```

Despawns the last registered entity.

## Entity Information

```rust
let entities: &[EntityId] = context.entities();
let exists: bool = context.is_entity_registered(entity);
let count: usize = context.entity_count();
```

## Components

```rust
context.add_component(entity, Transform::default())?;
let transform = context.get_component::<Transform>(entity);
let transform = context.get_component_mut::<Transform>(entity);
let has = context.has_component::<Transform>(entity);
let removed = context.remove_component::<Transform>(entity);
```

`add_component()` returns `Result<()>` and replaces an existing component of the same type. Invalid entities or assets return an error without changing the old component. For asset-owning components, it retains new assets before releasing the old references; manual retain/release is unnecessary.

`remove_component()` returns `true` if removed, or `false` if absent. It releases the component’s asset references and does not return the removed value.

### Mutable Access Restrictions

`get_component_mut()` and `get_component_pool_mut()` return `None` for `MeshRenderer`, `PendingPrimitiveMesh`, and `GltfModelRoot`. Queries requesting mutable access to these types return an empty iterator.

Read-only access remains available, including `query2_mut::<Transform, MeshRenderer>()`. Use `set_material()` to change materials or `add_component()` to replace a whole component.

Direct low-level access to `World` or `Resources` bypasses this reference management and these restrictions.

## Queries

Queries iterate over entities with specific components.

```rust
for (entity, transform) in context.query1::<Transform>() {
    log::debug!("{entity:?}: {:?}", transform.position);
}
```

```rust
for (_, transform) in context.query1_mut::<Transform>() {
    transform.position.y += 1.0;
}
```

```rust
let delta_seconds = context.delta_seconds();
for (_, transform, rotator) in context.query2_mut::<Transform, Rotator>() {
    transform.rotate(rotator.speed * delta_seconds);
}
```

Use `query2_mut_mut()` when both component types must be mutable.

```rust
for (_, transform, camera) in context.query2_mut_mut::<Transform, Camera>() {
    camera.target = transform.position;
}
```

The two component types must be different.

## Names

A name is unique among entities.

```rust
context.set_name(entity, "Player");
let player = context.find_entity_by_name("Player");
context.remove_name(entity);
```

Get all named entities.

```rust
let named_entities: Vec<(String, EntityId)> = context.get_all_named_entities();
```

## Tags

Tags are labels. Multiple entities can share the same tag.

```rust
context.set_tags(entity, ["Object", "Enemy"]);
let enemies = context.get_entities_by_tag("Enemy");
context.remove_tag(entity, "Enemy");
context.remove_tags(entity);
```

Get all tag/entity pairs.

```rust
let tagged_entities: Vec<(String, EntityId)> = context.get_all_taged_entities();
```

The API name is currently `get_all_taged_entities()`.

### Get MeshAssetId from Entity
You can get `MeshAssetId` from Entity. If specified Entity have mesh, 
then return `Some(MeshAssetId)`, else return `None`.

```rust
let mesh_asset_id: Option<MeshAssetId> = context.mesh_asset_id(entity);
```
## Context Difference

`SceneContext::spawn()` automatically adds `SceneOwned`.

`UpdateContext` and `CommandContext` use the default `spawn()`, so they do not add `SceneOwned` automatically.

If a command-created entity should be removed when the scene exits, add `SceneOwned` manually.

```rust
context.add_component(entity, SceneOwned { scene_id })?;
```
