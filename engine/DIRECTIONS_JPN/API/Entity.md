# Entity API

Entity APIは、ECS上のEntityを作成・削除し、Component、Name、Tagを操作するためのAPIです。
`SceneContext`、`UpdateContext`、`CommandContext`から呼び出すことができます。

```rust
use kani_volcano_engine::{EntityApi, Rotator};
use kani_volcano_math::Transform;

let entity = context.spawn();

context.add_component(entity, Transform::default())?;
context.add_component(
    entity,
    Rotator {
        speed: cgmath::vec3(0.0, 1.0, 0.0),
    },
)?;
```

Entityはゲーム内オブジェクトを表すIDです。
`spawn()`した直後のEntityは、まだComponentを持っていません。
Entityに機能や状態を持たせるには、`add_component()`でComponentを付与します。

## Entityの作成と削除

### Entity作成

```rust
let entity: EntityId = context.spawn();
```

新しいEntityを作成し、`EntityId`を返します。
`SceneContext`で作成したEntityには、自動的に`SceneOwned` Componentが付与されます。
そのため、Sceneの終了時に`despawn_scene_owned_entities()`でまとめて削除できます。

### Entity削除

```rust
let removed: bool = context.despawn(entity);
```

指定したEntityと、`ParentComponent`でつながる子孫を削除します。
Entityが存在する場合は`true`、存在しない場合は`false`を返します。

削除時には、そのEntityが持っていたComponentもすべて削除されます。
mesh・texture・glTFモデルの参照数も減らします。最後の参照がなくなり、`auto_release=true`の資産は解放され、GPU破棄が予約されます。

### 最後のEntity削除

```rust
let removed: bool = context.despawn_last();
```

登録されているEntityの最後の1つを削除します。
Entityが1つもない場合は`false`を返します。

Commandでは、例えば`DespawnLastCommand`がこのAPIを使っています。

```rust
context.despawn_last();
```

## Entity情報の取得

### Entity一覧

```rust
let entities: &[EntityId] = context.entities();
```

現在Worldに登録されているEntityの一覧を取得します。

### Entityの存在確認

```rust
let exists: bool = context.is_entity_registered(entity);
```

指定したEntityがWorldに登録されているかを確認します。

### Entity数

```rust
let count: usize = context.entity_count();
```

現在Worldに登録されているEntity数を取得します。

## Component操作

### Component追加

```rust
context.add_component(entity, Transform::default())?;
```

EntityにComponentを追加・上書きし、`Result<()>`を返します。Entityや参照先の資産が無効な場合はエラーになり、元のComponentは維持されます。

資産を持つComponentでは、新しい資産の参照数を増やしてから、上書きされるComponentの参照数を減らします。利用者によるretain/releaseは不要です。

### Component削除

```rust
let removed: bool = context.remove_component::<Transform>(entity);
```

Entityから指定した型のComponentを削除します。
削除できた場合は`true`、存在しない場合は`false`を返します。削除したComponentの資産参照数を減らします。Component自体は返しません。

### Component取得

```rust
let transform: Option<&Transform> = context.get_component::<Transform>(entity);
```

Entityが持つComponentを不変参照で取得します。

### Component変更

```rust
if let Some(transform) = context.get_component_mut::<Transform>(entity) {
    transform.position.x += 1.0;
}
```

Entityが持つComponentを可変参照で取得します。

### Componentの有無

```rust
let has_transform: bool = context.has_component::<Transform>(entity);
```

Entityが指定した型のComponentを持っているかを確認します。

### ComponentPool取得

```rust
let pool = context.get_component_pool::<Transform>();
let pool_mut = context.get_component_pool_mut::<Transform>();
```

指定した型のComponentPoolを取得します。
通常は`query1()`や`query2()`を使う方が扱いやすいですが、Pool全体を直接見たい場合に使えます。

### 可変アクセスの制限

`MeshRenderer`・`PendingPrimitiveMesh`・`GltfModelRoot`は資産の参照管理が必要なため、`get_component_mut()`と`get_component_pool_mut()`は`None`を返します。これらを可変取得するqueryは空のiteratorを返します。

読み取りは可能です。`query2_mut::<Transform, MeshRenderer>()`のように、管理対象を読み取り側に指定することもできます。Material変更は`set_material()`、Component全体の交換は`add_component()`を使用してください。

`World`や`Resources`を直接操作する低レベルAPIは、この参照管理と制限を迂回します。

## Query

Queryは、指定したComponentを持つEntityだけを列挙するためのAPIです。
System内で複数Entityをまとめて更新するときによく使います。

### 1種類のComponentを読む

```rust
for (entity, transform) in context.query1::<Transform>() {
    log::debug!("{entity:?}: {:?}", transform.position);
}
```

`Transform`を持つEntityだけを列挙します。

### 1種類のComponentを変更する

```rust
for (_, transform) in context.query1_mut::<Transform>() {
    transform.position.y += 1.0;
}
```

`Transform`を持つEntityを列挙し、Componentを変更します。

### 2種類のComponentを読む

```rust
for (entity, transform, camera) in context.query2::<Transform, Camera>() {
    log::debug!("{entity:?}: {:?} {:?}", transform.position, camera.target);
}
```

指定した2種類のComponentを両方持つEntityだけを列挙します。

### 片方を変更し、もう片方を読む

```rust
let delta_seconds = context.delta_seconds();
for (_, transform, rotator) in context.query2_mut::<Transform, Rotator>() {
    transform.rotate(rotator.speed * delta_seconds);
}
```

`RotatorSystem`ではこの形で、`Rotator`を読みながら`Transform`を更新しています。

### 2種類のComponentを両方変更する

```rust
for (_, transform, camera) in context.query2_mut_mut::<Transform, Camera>() {
    transform.position.z += 1.0;
    camera.target = transform.position;
}
```

`CameraSystem`ではこの形で、`Transform`と`Camera`を同時に更新しています。

`query2_mut()`と`query2_mut_mut()`では、同じComponent型を2回指定することはできません。
例えば`query2_mut::<Transform, Transform>()`のような呼び方はpanicします。

## Name

NameはEntityに一意の名前を付けるためのComponentです。

### 名前を設定する

```rust
let ok: bool = context.set_name(entity, "Player");
```

Entityに名前を設定します。
同じ名前を持つEntityが既に存在する場合は`false`を返します。

### 名前でEntityを探す

```rust
let player: Option<EntityId> = context.find_entity_by_name("Player");
```

指定した名前を持つEntityを検索します。
見つからない場合は`None`を返します。

### 名前を削除する

```rust
let removed: bool = context.remove_name(entity);
```

EntityからName Componentを削除します。
削除できた場合は`true`、名前が付いていなかった場合は`false`を返します。

### 名前付きEntityをすべて取得する

```rust
let named_entities: Vec<(String, EntityId)> = context.get_all_named_entities();
```

すべての名前付きEntityを取得します。
`DebugMonitor`では、デバッグ表示用にこのAPIを使用しています。

## Tag

TagはEntityに複数の分類ラベルを付けるためのComponentです。
Nameとは違い、同じTagを複数Entityに付けることができます。

### Tagを設定する

```rust
let ok: bool = context.set_tags(entity, ["Object", "Enemy"]);
```

EntityにTag一覧を設定します。
内部では`Tags` Componentが追加されます。

### TagでEntityを探す

```rust
let enemies: Vec<EntityId> = context.get_entities_by_tag("Enemy");
```

指定したTagを持つEntityをすべて取得します。
見つからない場合は空の`Vec`を返します。

### 1つのTagを削除する

```rust
let removed: bool = context.remove_tag(entity, "Enemy");
```

Entityから指定したTagを1つ削除します。
削除できた場合は`true`、対象のTagがなかった場合は`false`を返します。

### Tag Componentを削除する

```rust
let removed: bool = context.remove_tags(entity);
```

Entityから`Tags` Componentごと削除します。

### Tag付きEntityをすべて取得する

```rust
let tagged_entities: Vec<(String, EntityId)> = context.get_all_taged_entities();
```

すべてのTagとEntityの組み合わせを取得します。
1つのEntityが複数Tagを持つ場合は、Tagごとに1件ずつ返ります。

## Contextごとの違い

`EntityApi`自体の関数は、基本的に`SceneContext`、`UpdateContext`、`CommandContext`で同じように使えます。

ただし、`SceneContext`の`spawn()`だけは特別です。
`SceneContext`で作成したEntityには`SceneOwned`が自動付与されます。

```rust
fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
    let entity = context.spawn();
    context.set_name(entity, "SceneObject");

    Ok(())
}

fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
    context.despawn_scene_owned_entities();

    Ok(())
}
```

もし、`SceneContext.spawn()`以外で`SceneOwned`を付与したい場合は、`SceneContext`から`scene_id()`を呼びだすことで、そのSceneのIdを得ることが出来ます。その後、Entityに`SceneOwned`コンポーネントを付与することで、EntityがScene特有のものになります。
```rust
context.bind_input_command(
    KeyCode::Digit1,
    InputTrigger::Pressed,
    CreateRectangleCommand{
        len: 0.2,
        color: vec3(1.0,0.0,0.0),
        alpha: 1.0,
        texture: None,
        pipeline_key: PipelineKey::Mesh3D,
        scene_id: context.scene_id(),
    }
);

...
struct CreateRectangleCommand{
    len: f32,
    color: Vec3,
    alpha: f32,
    texture: Option<&'static str>,
    pipeline_key: PipelineKey,
    scene_id: SceneId,
}

impl Command for CreateRectangleCommand{
    fn id(&self) -> String{
        format!("create_rectangle")
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()>{
        let position=mouse_position_on_spawn_plane(context);
        let rectangle=context.spawn_rectangle_3d(
            ...
        )?;
        context.add_component(
            rectangle,
            SceneOwned{
                scene_id: self.scene_id,
            }
        )?;
        Ok(())
    }
}
```


`UpdateContext`では、毎フレームのSystem更新中にEntityやComponentを操作できます。

```rust
impl UpdateSystem for MoveSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let delta_seconds = context.delta_seconds();
        for (_, transform, velocity) in context.query2_mut::<Transform, Velocity>() {
            transform.position += velocity.value * delta_seconds;
        }

        Ok(())
    }
}
```

`CommandContext`では、入力に紐づいたCommandの実行時にEntityやComponentを操作できます。

```rust
impl Command for DespawnLastCommand {
    fn id(&self) -> String {
        "despawn_last".to_string()
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        context.despawn_last();

        Ok(())
    }
}
```

## よく使う流れ

### Entityを作ってComponentを付ける

```rust
let entity = context.spawn();

context.add_component(entity, Transform::default())?;
context.add_component(entity, Visibility::default())?;
context.set_name(entity, "Player");
context.set_tags(entity, ["Object", "Player"]);
```

### Systemから対象Entityを更新する

```rust
let delta_seconds = context.delta_seconds();
for (_, transform, rotator) in context.query2_mut::<Transform, Rotator>() {
    transform.rotate(rotator.speed * delta_seconds);
}
```

### 名前やTagでEntityを取得する

```rust
if let Some(player) = context.find_entity_by_name("Player") {
    if context.has_component::<Transform>(player) {
        log::debug!("Player has Transform");
    }
}

for enemy in context.get_entities_by_tag("Enemy") {
    context.despawn(enemy);
}
```

### MeshAssetIdの取得
Entityを指定して、MeshAssetIdを取得することができます。もし、そのEnitityがメッシュを持っていなかったら`None`を返し、メッシュを持っていたら`Some(MeshAssetId)`を返します。

```rust
let mesh_asset_id: Option<MeshAssetId> = context.mesh_asset_id(entity);
```

## 注意点

`EntityId`はEntityを識別するためのIDです。
Entityの実体や状態はComponentにあります。

`despawn()`すると、そのEntityが持つComponentも削除されます。
資産の解放は参照数と`auto_release`に従います。

Nameは重複できません。
同じ名前を登録しようとすると`set_name()`は`false`を返します。

Tagは複数Entityで共有できます。
同じTagを持つEntityをまとめて取得したい場合は`get_entities_by_tag()`を使います。

`get_all_taged_entities()`は関数名が`taged`になっています。
英語としては`tagged`ですが、現在のAPI名に合わせてこの名前を使います。
