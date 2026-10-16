# JobApi

並列処理とは、複数の CPU コアなどを利用して、同時に複数の処理を実行することです。並行処理とは、複数の処理を進行させることです。並行処理は、1つの CPU コアで処理を切り替えながら実行する場合も、複数のコアで並列に実行する場合もあります。

並列処理では、例えば、コア1で物理の衝突判定を行いながら、コア2で AI の計算を行えます。また、大量の衝突判定を2つのコアで分担することもできます。ただし、タスク間に依存関係がある場合は、実行順序と同期を適切に制御する必要があります。

並行処理では、ファイルのダウンロード中に別の処理を進めることができます。I/O 待ちなどで CPU を使用しない処理がある場合、その待ち時間に別の処理を実行することで CPU を効率的に活用できます。ただし、ブロッキング I/O を Job 内で呼んでも、自動的に fiber の待機に変換されるわけではありません。

## Worker と Job fiber

KaniVolcanoEngine の標準設定では、利用可能な並列数から1を引いた数の Worker スレッドを作成します。Worker 数は最低1です。また、初期 Job fiber 数は Worker 数の4倍です。

Worker は Job（処理の1単位）を実行するスレッドです。各 Worker は、起動した自分のスレッドを manager fiber に変換します。manager fiber は Worker 専用であり、共有する Job fiber のプールとは別です。

投入した Job は共有キューに入り、空いている Job fiber に割り当てられます。Worker は manager fiber から Job fiber に切り替えて処理を実行します。Job が完了するか、別の Job の完了を待つ場合は manager fiber に戻り、別の Job を実行できます。待機していた Job fiber は、依存する Job の完了後に別の Worker で再開される場合もあります。空き Job fiber が不足した場合は追加作成されます。

fiber の切り替えはアプリケーション側で行います。ただし、Job の作成、キューへの投入、同期、fiber の切り替えにもコストがあるため、すべての処理を並列にすれば速くなるわけではありません。

![Worker と Job fiber の実行の流れ](../../DIRECTIONS_ENG/assets/job_system_explanation.png)

## 使用例

`Rotator` component が付与されたすべてのオブジェクトを、並列に回転させる `ParallelRotatorSystem` の例です。エンジンが提供する `Rotator` は、次のように回転速度を保持します。

```rust
use cgmath::Vector3;
use kani_volcano_engine::Component;

pub struct Rotator {
    pub speed: Vector3<f32>,
}

impl Component for Rotator {}
```

以下の例では、エンジンが提供する `Rotator` を import して使います。

```rust
use anyhow::Result;
use kani_volcano_engine::{EntityApi, JobApi, Rotator, TimeApi, UpdateContext, UpdateSystem};
use kani_volcano_math::Transform;

#[derive(Default)]
pub struct ParallelCalculatorSystem {}

impl UpdateSystem for ParallelCalculatorSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        // component を借用する前に、JobSystemHandle を取得する。
        let jobs = context.job_system()?;
        let dt = context.delta_seconds();

        let mut entries: Vec<_> = context.query2_mut::<Transform, Rotator>().collect();

        unsafe {
            jobs.scope(|scope| -> Result<()> {
                // 子 Job の投入前の処理は、呼び出し元のスレッドで実行される。

                // これから投入する子 Job の完了を管理するグループを作成する。
                let group = scope.group();

                for chunk in entries.chunks_mut(256) {
                    scope.spawn(&group, "rotate_chunk", move || {
                        // このクロージャは Worker 上で実行される。
                        for (_, transform, rotator) in chunk {
                            transform.rotate(rotator.speed * dt);
                        }
                    })?;
                }

                // このグループに属するすべての Job の完了を待つ。
                scope.wait(&group)?;

                // 待機後の処理は、呼び出し元のスレッドで実行される。

                Ok(())
            })
        }??;
        Ok(())
    }
}
```

`context.job_system()` は、エンジンの JobSystem を利用するための `JobSystemHandle` を返します。Scheduler は update / fixed update stage の実行中、そのスレッドの TLS に handle を登録します。handle は Context を借用せずに取得できるため、その後で component を可変借用できます。

1. `jobs.scope(...)`：借用したデータを使う Job を作成できる範囲を開始します。scope の終了前に、その scope から投入したすべての Job の完了が保証されます。明示的な `wait()` を省略した場合も同様です。
2. `scope.group()`：投入する Job の完了をまとめて管理するグループを作成します。
3. `scope.spawn(&group, name, func)`：`func`（`FnOnce() + Send`）を Job として共有キューに投入し、指定したグループに登録します。`name` は計測用の名前です。同じ名前の Job も、それぞれ別の JobId で識別されます。この例では、重複しない256件ずつの範囲を各 Job に渡します。
4. `scope.wait(&group)`：指定したグループに属するすべての Job の完了を待ちます。この呼び出しでグループは追加投入を禁止する状態になるため、以降の Job には新しいグループを使います。

例えば、処理 A の後に Job B・C・D を並列に実行し、その結果を使って処理 E を行いたい場合、B・C・D を同じグループに登録します。A を実行してから子 Job を投入し、`scope.wait(&group)?` の後に E を実行します。scope のクロージャ自体は自動的に Job になるわけではありません。

現在の Scheduler から呼ばれる `update()` は main thread 上で実行されます。上の例では、main thread が子 Job の完了を待ち、完了後に後続処理を続けます。Worker 上の Job が `wait()` を呼ぶ場合は、その Job fiber が停止し、Worker はほかの Job を実行できます。

最後の `??` は、`scope()` 自体のエラーと、そのクロージャが返す `Result` の両方を伝播させます。

## unsafe の条件

現在の `scope()` と `wait()` は `unsafe` です。Worker 上で呼ぶ場合、停止した fiber が別の Worker で再開される可能性があるため、呼び出し元のフレームも含むスタック全体の値と参照が、その移動に対応している必要があります。スレッドに依存するガード、TLS への参照、`Send` でない値などを、待機をまたいで保持しないでください。`Send` なクロージャでも、その内部で作成するローカル変数まで移動可能とは限りません。

また、自分自身を含むグループの完了待ちや、循環する依存関係は避けてください。

## 並列化に向いている処理

この `ParallelRotatorSystem` は使い方を示す例であり、軽い回転処理の高速化には向いていません。各オブジェクトの更新は、回転速度と時間の乗算、および回転角度への加算が中心です。更新時に行列計算を行っているわけではありません。

1 Job あたりの計算量が小さいと、Job の投入、同期、fiber の切り替えによるコストが、並列化による時間短縮を上回ります。さらに、この例では query 結果を `Vec` に収集する処理も直列です。今回の測定では、並列版より直列の `RotatorSystem` の方が高速でした。

並列化する場合は、1 Job が担当する件数を増やして Job 数を減らすか、次のような計算量の多い処理を対象にし、実測して比較してください。

- アニメーションの補間やボーン行列の計算
- AI の経路探索
- 独立した衝突候補の判定
- 地形や mesh の頂点・index・法線の生成

Job 間で可変データへの参照が重複しないように分割し、GPU 操作などスレッドに制約のある処理は、計算結果を集約した後で適切なスレッドから実行します。


## プロファイリング

KaniVolcanoEngine では、Worker の数や Job の実行履歴を取得するための API を提供しています。これらの API を使うことで、ユーザーが作成した system から実行状況を確認したり、タイムラインとして可視化したりできます。

履歴は `JobRunSegment` 単位で記録されます。これは、Job がある Worker 上で実行された1つの区間を表します。待機や再開によって実行区間が分かれた場合も、同じ JobId で追跡できます。取得できるのは、保存上限内に残っている記録です。

### Worker 数の取得

```rust
let worker_count: usize = context.worker_count()?;
```

JobSystem が持つ Worker の数を取得します。Worker の ID は、`0` から `worker_count - 1` までです。

### Worker ID を指定した実行履歴の取得

```rust
let segments = context.worker_history(0)?;
```

指定した Worker の実行履歴を、古い順に並んだ `VecDeque<JobRunSegment>` として取得します。この例では、Worker 0 の履歴を取得しています。存在しない Worker ID を指定するとエラーになります。

取得した一覧は内部の履歴をコピーしたものです。一覧を変更しても、JobSystem が保持する記録には影響しません。

### Job 名を指定した実行履歴の取得

```rust
let segments: Vec<JobRunSegment> =
    context.get_job_segments_from_name("rotate_chunk")?;
```

すべての Worker の履歴から、指定した名前と完全に一致する Job の実行区間を検索します。結果は開始時刻順に並び、一致する記録がなければ空の `Vec` を返します。同じ名前で投入した別の Job も含まれるため、個々の Job を区別するときは JobId を使います。

### 実行履歴の可視化例

![Worker ごとの Job 実行履歴](../../../assets/job_system_profiler.png)

上の画像は、前述の `ParallelCalculatorSystem` が投入した Job の実行履歴を可視化したものです。Worker ごとに1行を設け、共通の時間軸上に実行区間を表示しています。

`ThreadProfilerSystem` は、標準設定では直近1秒間の履歴を100msごとに取得して表示を更新します。描画用の entity は再利用し、短い実行区間は一定幅の時間区間にまとめて表示することで、表示処理の負荷を抑えています。このため、色の付いた部分は「その時間帯に実行記録がある」ことを示し、その区間全体で Worker が動き続けていたことを示すものではありません。また、OS によるスレッドの中断時間も実行区間に含まれるため、厳密な CPU 使用率とは異なります。

以下に、可視化を行う system の実装と、Scene への登録例を示します。

`thread_profiler_system.rs`

```rust
use super::{UpdateContext, UpdateSystem};
use crate::{
    AssetApi, EntityApi, EntityId, FontAssetId, JobApi, JobRunSegment, Material, MeshAssetId,
    MeshRenderer, ObjectApi, SceneId, SceneOwned, Text, TextPipeline, Visibility,
};
use anyhow::{Result, ensure};
use cgmath::{vec2, vec3};
use kani_volcano_math::Transform;
use renderer_vulkan::{PipelineKey, VertexLayout};
use std::time::{Duration, Instant};

const LEFT: f32 = -0.55;
const RIGHT: f32 = 0.95;
const TOP: f32 = 0.8;
const BOTTOM: f32 = -0.65;
// Bound draw objects independently of the much larger retained job history.
const TOTAL_BINS: usize = 512;
const MAX_BINS_PER_WORKER: usize = 256;

/// Screen-space timeline of retained, completed worker dispatches.
/// Both assets must be centered white 1x1 rectangles with Ui2D vertex layout.
/// line_mesh_id is drawn as a thin rectangle, not a DebugLine3D line-list mesh.
///
/// Register with add_update_system, after the systems being observed. Labels use
/// the first loaded font, if any. Set scene_id for scene-owned overlay entities.
/// Bars are quantized to a bounded number of time bins; colors group job names.
/// This is a timeline of recorded job dispatches, not measured OS CPU utilization.
pub struct ThreadProfilerSystem {
    pub rect_mesh_id: MeshAssetId,
    pub line_mesh_id: MeshAssetId,
    pub refresh_interval: Duration,
    pub time_window: Duration,
    pub scene_id: Option<SceneId>,
    last_refresh: Option<Instant>,
    rectangles: Vec<EntityId>,
    lines: Vec<EntityId>,
    labels: Vec<EntityId>,
}

impl ThreadProfilerSystem {
    pub fn new(rect_mesh_id: MeshAssetId, line_mesh_id: MeshAssetId) -> Self {
        Self {
            rect_mesh_id,
            line_mesh_id,
            refresh_interval: Duration::from_millis(100),
            time_window: Duration::from_secs(1),
            scene_id: None,
            last_refresh: None,
            rectangles: Vec::new(),
            lines: Vec::new(),
            labels: Vec::new(),
        }
    }

    /// Remove this system's overlay before removing the system itself.
    /// Alternatively, assign scene_id and let scene cleanup remove the entities.
    pub fn clear(&mut self, context: &mut UpdateContext<'_>) {
        for id in self
            .rectangles
            .drain(..)
            .chain(self.lines.drain(..))
            .chain(self.labels.drain(..))
        {
            context.despawn(id);
        }
        self.last_refresh = None;
    }

    fn validate_mesh(context: &UpdateContext<'_>, id: MeshAssetId) -> Result<()> {
        let layout = context
            .mesh_assets()
            .find(|(asset_id, _)| *asset_id == id)
            .map(|(_, asset)| asset.handle.vertex_layout);
        ensure!(
            layout == Some(VertexLayout::Ui2D),
            "ThreadProfilerSystem mesh {:?} must be a centered white 1x1 Ui2D rectangle",
            id
        );
        Ok(())
    }

    fn tag(context: &mut UpdateContext<'_>, id: EntityId, scene_id: Option<SceneId>) {
        context.set_tags(id, ["ThreadProfiler"]);
        if let Some(scene_id) = scene_id {
            context.add_component(id, SceneOwned { scene_id });
        }
    }

    // Reuse the supplied mesh through ObjectApi; never create GPU meshes here.
    fn spawn_rectangle_from_mesh(
        context: &mut UpdateContext<'_>,
        mesh: MeshAssetId,
        scene_id: Option<SceneId>,
    ) -> Result<EntityId> {
        let id = context.spawn_primitive_from_mesh(
            mesh,
            Material {
                pipeline_key: PipelineKey::Ui2D,
                ..Material::default()
            },
            Transform::default(),
        )?;
        Self::tag(context, id, scene_id);
        Ok(id)
    }

    fn rectangle(
        context: &mut UpdateContext<'_>,
        mesh: MeshAssetId,
        pool: &mut Vec<EntityId>,
        used: &mut usize,
        scene_id: Option<SceneId>,
        center: cgmath::Vector2<f32>,
        size: cgmath::Vector2<f32>,
        color: cgmath::Vector3<f32>,
    ) -> Result<()> {
        let existing = pool.get(*used).copied().filter(|id| {
            context
                .get_component::<MeshRenderer>(*id)
                .is_some_and(|renderer| renderer.asset_id == Some(mesh))
                && context.has_component::<Transform>(*id)
                && context.has_component::<Visibility>(*id)
        });
        let id = if let Some(id) = existing {
            id
        } else {
            if let Some(&old) = pool.get(*used) {
                context.despawn(old);
            }
            let id = Self::spawn_rectangle_from_mesh(context, mesh, scene_id)?;
            if *used == pool.len() {
                pool.push(id);
            } else {
                pool[*used] = id;
            }
            id
        };
        // Ui2D uses position.yz and scale.yz; its vertex shader ignores rotation.
        *context.get_component_mut::<Transform>(id).unwrap() = Transform {
            position: vec3(0.0, center.x, center.y),
            scale: vec3(1.0, size.x, size.y),
            ..Transform::default()
        };
        context
            .get_component_mut::<MeshRenderer>(id)
            .unwrap()
            .material
            .color = color;
        context
            .get_component_mut::<Visibility>(id)
            .unwrap()
            .is_visible = true;
        *used += 1;
        Ok(())
    }

    fn label(
        &mut self,
        context: &mut UpdateContext<'_>,
        used: &mut usize,
        font: FontAssetId,
        content: String,
        x: f32,
        y: f32,
        font_size: f32,
    ) {
        let id = if let Some(id) = self.labels.get(*used).copied().filter(|id| {
            context.has_component::<Text>(*id)
                && context.has_component::<Transform>(*id)
                && context.has_component::<Visibility>(*id)
        }) {
            id
        } else {
            if let Some(&old) = self.labels.get(*used) {
                context.despawn(old);
            }
            let id = context.spawn();
            Self::tag(context, id, self.scene_id);
            context.add_component(id, Transform::default());
            context.add_component(id, Visibility::default());
            context.add_component(
                id,
                Text {
                    content: String::new(),
                    font,
                    font_size,
                    line_height: font_size,
                    color: vec3(0.9, 0.9, 0.9),
                    alpha: 1.0,
                    text_pipeline: TextPipeline::Ui2D,
                },
            );
            if *used == self.labels.len() {
                self.labels.push(id);
            } else {
                self.labels[*used] = id;
            }
            id
        };
        let text = context.get_component_mut::<Text>(id).unwrap();
        if text.content != content {
            text.content = content;
        }
        text.font = font;
        text.font_size = font_size;
        text.line_height = font_size;
        // TextUi2D anchors use XY, with positive Y downward (unlike Ui2D rectangles).
        context.get_component_mut::<Transform>(id).unwrap().position = vec3(x, -y, 0.0);
        context
            .get_component_mut::<Visibility>(id)
            .unwrap()
            .is_visible = true;
        *used += 1;
    }

    fn hide_unused(context: &mut UpdateContext<'_>, pool: &[EntityId], used: usize) {
        for &id in &pool[used..] {
            if let Some(visibility) = context.get_component_mut::<Visibility>(id) {
                visibility.is_visible = false;
            }
        }
    }
}

impl UpdateSystem for ThreadProfilerSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let now = Instant::now();
        if self
            .last_refresh
            .is_some_and(|last| now.duration_since(last) < self.refresh_interval)
        {
            return Ok(());
        }
        ensure!(
            !self.time_window.is_zero(),
            "profiler time_window must be positive"
        );
        Self::validate_mesh(context, self.rect_mesh_id)?;
        Self::validate_mesh(context, self.line_mesh_id)?;

        let jobs = context.job_system()?;
        let worker_count = context.worker_count()?;
        // Capture once so every worker uses the same time window.
        let end = jobs.profiler().elapsed();
        let font = context.font_assets().next().map(|(id, _)| id);
        let row_height = (TOP - BOTTOM) / worker_count.max(1) as f32;
        let bin_count = (TOTAL_BINS / worker_count.max(1)).clamp(1, MAX_BINS_PER_WORKER);
        let mut bins = vec![None; bin_count];
        let (mut rectangles, mut lines, mut labels) = (0, 0, 0);

        for worker in 0..worker_count {
            let history = context.worker_history(worker)?;
            fill_bins(history.iter(), end, self.time_window, &mut bins);
            let y = TOP - (worker as f32 + 0.5) * row_height;
            Self::rectangle(
                context,
                self.rect_mesh_id,
                &mut self.rectangles,
                &mut rectangles,
                self.scene_id,
                vec2((LEFT + RIGHT) * 0.5, y),
                vec2(RIGHT - LEFT, row_height * 0.75),
                vec3(0.06, 0.07, 0.09),
            )?;

            let mut bin = 0;
            while bin < bin_count {
                let Some(color_index) = bins[bin] else {
                    bin += 1;
                    continue;
                };
                let start = bin;
                while bin < bin_count && bins[bin] == Some(color_index) {
                    bin += 1;
                }
                let left = LEFT + (RIGHT - LEFT) * start as f32 / bin_count as f32;
                let right = LEFT + (RIGHT - LEFT) * bin as f32 / bin_count as f32;
                Self::rectangle(
                    context,
                    self.rect_mesh_id,
                    &mut self.rectangles,
                    &mut rectangles,
                    self.scene_id,
                    vec2((left + right) * 0.5, y),
                    vec2(right - left, row_height * 0.65),
                    job_color(color_index),
                )?;
            }
            if let Some(font) = font {
                self.label(
                    context,
                    &mut labels,
                    font,
                    format!("Worker {worker}"),
                    -0.98,
                    y + row_height * 0.15,
                    (row_height * 300.0).clamp(4.0, 12.0),
                );
            }
        }

        // Six grid lines, with labels relative to the captured window end.
        for tick in 0..=5 {
            let fraction = tick as f32 / 5.0;
            let x = LEFT + (RIGHT - LEFT) * fraction;
            Self::rectangle(
                context,
                self.line_mesh_id,
                &mut self.lines,
                &mut lines,
                self.scene_id,
                vec2(x, (TOP + BOTTOM) * 0.5),
                vec2(0.001, TOP - BOTTOM + 0.02),
                vec3(0.25, 0.27, 0.30),
            )?;
            if let Some(font) = font {
                let ms = self.time_window.as_secs_f64() * 1000.0 * (1.0 - f64::from(fraction));
                self.label(
                    context,
                    &mut labels,
                    font,
                    format!("-{ms:.0} ms"),
                    x - 0.04,
                    BOTTOM - 0.04,
                    11.0,
                );
            }
        }
        Self::rectangle(
            context,
            self.line_mesh_id,
            &mut self.lines,
            &mut lines,
            self.scene_id,
            vec2((LEFT + RIGHT) * 0.5, BOTTOM - 0.01),
            vec2(RIGHT - LEFT, 0.002),
            vec3(0.5, 0.5, 0.5),
        )?;
        Self::hide_unused(context, &self.rectangles, rectangles);
        Self::hide_unused(context, &self.lines, lines);
        Self::hide_unused(context, &self.labels, labels);
        self.last_refresh = Some(Instant::now());
        Ok(())
    }
}

// Quantize retained intervals to display bins so thousands of jobs do not become
// thousands of Vulkan draw objects. A filled bin means some work occurred there,
// not that the worker was occupied for the entire bin. Later intervals win ties.
fn fill_bins<'a>(
    segments: impl Iterator<Item = &'a JobRunSegment>,
    end: Duration,
    window: Duration,
    bins: &mut [Option<usize>],
) {
    bins.fill(None);
    let seconds = window.as_secs_f64();
    let window_start = end.as_secs_f64() - seconds;
    let bin_count = bins.len();
    for segment in segments {
        let left = ((segment.started_at.as_secs_f64() - window_start) / seconds).clamp(0.0, 1.0);
        let right = ((segment.ended_at.as_secs_f64() - window_start) / seconds).clamp(0.0, 1.0);
        if right <= left {
            continue;
        }
        let first = (left * bin_count as f64).floor() as usize;
        let last = ((right * bin_count as f64).ceil() as usize).min(bin_count);
        let hash = segment.job.name.bytes().fold(0u64, |hash, byte| {
            hash.wrapping_mul(31).wrapping_add(u64::from(byte))
        });
        bins[first..last].fill(Some((hash % 6) as usize));
    }
}

fn job_color(index: usize) -> cgmath::Vector3<f32> {
    [
        vec3(0.2, 0.7, 1.0),
        vec3(0.3, 0.9, 0.5),
        vec3(1.0, 0.7, 0.2),
        vec3(0.8, 0.4, 1.0),
        vec3(1.0, 0.4, 0.5),
        vec3(0.2, 0.9, 0.9),
    ][index]
}
```

`parallel_calculator_scene.rs`

```rust
// Rotation work per job is too small to offset job scheduling, synchronization,
// and fiber switching overhead. Collecting query results also runs serially,
// making this implementation slower than RotatorSystem in our measurements.
use super::ChangeSceneCommand;
use crate::system::ParallelRotatorSystem;
use anyhow::{Result, anyhow};
use cgmath::{vec2, vec3};
use kani_volcano_engine::ThreadProfilerSystem;
use kani_volcano_engine::prelude::*;
use rand::{RngExt, rngs::ThreadRng};
use winit::keyboard::KeyCode;

pub struct ParallelCalculatorScene {
    scene_id: SceneId,
    main_cube: Option<EntityId>,
    ground: f32,
    rng: ThreadRng,
    cubes_created: bool,
}

impl Scene for ParallelCalculatorScene {
    fn name(&self) -> String {
        "ParallelCalculatorScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        self.scene_id = context.scene_id();
        self.main_cube = None;
        self.cubes_created = false;
        context.set_skybox("default")?;
        self.create_camera(context)?;
        self.create_fundation(context)?;
        self.create_texts(context)?;
        self.create_rotate_entities(context)?;
        //self.spawn_rotate_primitive(context)?;

        self.add_update_systems(context);
        self.add_thread_profiler(context)?;
        self.bind_input_commands(context);
        Ok(())
    }

    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        /*if self.cubes_created {
            return Ok(());
        }
        let Some(cube) = self.main_cube else {
            return Ok(());
        };
        let Some(mesh_id) = context.mesh_asset_id(cube) else {
            return Ok(());
        };
        self.create_rotate_cubes(context, mesh_id)?;
        self.cubes_created = true;*/
        Ok(())
    }

    fn on_exit(&mut self, _context: &mut SceneContext<'_>) -> Result<()> {
        Ok(())
    }
}

impl ParallelCalculatorScene {
    fn create_camera(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let camera = context.spawn();
        context.add_component(camera, Transform::default());
        let success = context.add_component(
            camera,
            Camera {
                up: vec3(0.0, 0.0, 1.0),
                // A valid initial direction is needed before CameraSystem's first update.
                target: vec3(-1.0, 0.0, 0.0),
                fov_y: 45.0,
                near: 0.1,
                far: 100.0,
                yaw: std::f32::consts::PI,
                pitch: 0.0,
            },
        );

        if !success {
            Err(anyhow!("failed to create Camera"))
        } else {
            Ok(())
        }
    }

    fn create_fundation(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.spawn_rectangle_3d(
            vec3(0.0, 0.0, self.ground),
            40.0,
            40.0,
            vec3(0.0, -90.0, 0.0),
            vec3(0.5, 0.5, 0.5),
            1.0,
            None,
            PipelineKey::Mesh3D,
        )?;

        Ok(())
    }

    fn spawn_rotate_primitive(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let size = 0.5;
        let cube = context.spawn_cube_3d(
            vec3(0.0, 0.0, 0.0),
            size,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 1.0),
            1.0,
            None,
            PipelineKey::Mesh3D,
        )?;

        self.main_cube = Some(cube);
        if let Some(visibility) = context.get_component_mut::<Visibility>(cube) {
            visibility.is_visible = false;
        }
        Ok(())
    }

    fn create_rotate_entities(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        for _ in 0..100_000 {
            let entity = context.spawn();

            context.add_component(entity, Transform::default());
            context.add_component(
                entity,
                Rotator {
                    speed: vec3(60.0, 0.0, 0.0),
                },
            );
            context.add_component(
                entity,
                SceneOwned {
                    scene_id: self.scene_id,
                },
            );
        }

        Ok(())
    }
    fn create_rotate_cubes(
        &mut self,
        context: &mut UpdateContext<'_>,
        mesh_id: MeshAssetId,
    ) -> Result<()> {
        let texture = context.default_texture();
        for _ in 0..200 {
            let height = self.rng.random_range(1.0..5.0);
            let width = self.rng.random_range(0.5..2.0);
            let z = self.rng.random_range(0.0..5.0);
            let y = self.rng.random_range(-17.0..17.0);
            let x = self.rng.random_range(-17.0..17.0);
            let rot = self.rng.random_range(-60.0..60.0);

            let obj = context.spawn_primitive_from_mesh(
                mesh_id,
                Material {
                    color: vec3(0.7, 0.7, 0.7),
                    alpha: 1.0,
                    use_texture: false,
                    texture: texture,
                    pipeline_key: PipelineKey::Mesh3D,
                },
                Transform {
                    position: vec3(x, y, self.ground + z),
                    rotation: vec3(0.0, 0.0, 0.0),
                    scale: vec3(width, width, height),
                },
            )?;
            context.add_component(
                obj,
                Rotator {
                    speed: vec3(rot, 0.0, 0.0),
                },
            );
            context.set_tags(obj, ["rotator"]);
            context.add_component(
                obj,
                SceneOwned {
                    scene_id: self.scene_id,
                },
            );
        }
        Ok(())
    }

    fn create_texts(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.spawn_text_3d(
            "eng_font",
            "ParallelRotatorScene",
            100.0,
            100.0,
            vec3(-3.0, -3.0, 5.0),
            vec3(0.01, 0.01, 0.01),
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 1.0),
            1.0,
        )?;
        Ok(())
    }

    fn add_update_systems(&mut self, context: &mut SceneContext<'_>) {
        context.add_update_system("camera", CameraSystem);
        context
            .add_fixed_update_system("parallel_rotator_system", ParallelRotatorSystem::default());
    }

    fn add_thread_profiler(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        // Both rectangles and grid lines reuse centered white 1x1 Ui2D meshes.
        // SceneContext creates their GPU meshes during the following render stage.
        let mut mesh_entities = Vec::new();
        for _ in 0..2 {
            let entity = context.spawn_rectangle_2d(
                vec2(0.0, 0.0),
                1.0,
                1.0,
                0.0,
                vec3(1.0, 1.0, 1.0),
                1.0,
                None,
            )?;
            if let Some(visibility) = context.get_component_mut::<Visibility>(entity) {
                visibility.is_visible = false;
            }
            mesh_entities.push(entity);
        }
        context.add_update_system(
            "thread_profiler",
            ThreadProfilerBootstrap {
                mesh_entities: [mesh_entities[0], mesh_entities[1]],
                scene_id: self.scene_id,
                profiler: None,
            },
        );
        Ok(())
    }
}

// Resolve deferred mesh assets once, then delegate every update to the profiler.
pub struct ThreadProfilerBootstrap {
    pub mesh_entities: [EntityId; 2],
    pub scene_id: SceneId,
    pub profiler: Option<ThreadProfilerSystem>,
}

impl UpdateSystem for ThreadProfilerBootstrap {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        if self.profiler.is_none() {
            let Some(rect_mesh_id) = context.mesh_asset_id(self.mesh_entities[0]) else {
                return Ok(());
            };
            let Some(line_mesh_id) = context.mesh_asset_id(self.mesh_entities[1]) else {
                return Ok(());
            };
            let mut profiler = ThreadProfilerSystem::new(rect_mesh_id, line_mesh_id);
            profiler.scene_id = Some(self.scene_id);
            self.profiler = Some(profiler);
        }
        self.profiler.as_mut().unwrap().update(context)
    }
}

impl Default for ParallelCalculatorScene {
    fn default() -> Self {
        Self {
            scene_id: SceneId(0),
            main_cube: None,
            ground: -1.0,
            rng: rand::rng(),
            cubes_created: false,
        }
    }
}
```


`ThreadProfilerSystem` は、KaniVolcanoEngine が標準で提供しています。`use kani_volcano_engine::prelude::*;` で利用できるため、使用するために同じ system を自分で実装する必要はありません。上の system の実装は、内部の動作を説明するために掲載しています。
