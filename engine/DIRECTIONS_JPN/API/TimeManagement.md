# TimeManagerApi

`TimeManagementApi` は `App`、`UpdateContext`、`CommandContext` から利用できます。

```rust
use kani_volcano_engine::TimeManagementApi;

app.pause(); // 一時停止
app.step(1); // 1フレーム進む
app.resume(); // 再開
app.toggle_pause(); // 現在の停止状態を切り替える
```


`context.add_fixed_update_system("name",System)`で登録した、固定フレームレートで更新されるシステムに対して、上記のAPIを使用することで**停止**、**指定フレーム実行**、**再開**をすることが出来ます。

例えば、
```rust
context.add_update_system("camera", CameraSystem);
context.add_update_system(
    "audio",
    AudioSystem {
        buses: self.buses.clone(),
    },
);
context.add_fixed_update_system("rotator", MoveRotateSystem);
```
の場合は、`MoveRotatorSystem`だけに適用されます。

```rust
context.bind_debug_input_command(
    KeyCode::F6,
    InputTrigger::Pressed,
    DebugPauseCommand,
)
```

このように`bind_debug_input_command`でデバッグ用のコマンドとして、キーと紐づけることで停止や再開を扱えます。この例では、`F6`キーを押すと停止、再開を切り替えられ、奇数回目に`F6`キーを押すと、回転しているオブジェクトが停止、偶数回目に`F6`キーを押すと再開します。

`DebugPauseCommand`はKaniVolcanoEngineで用意されています。
```rust
pub struct DebugPauseCommand;

impl Command for DebugPauseCommand{
    fn id(&self) -> String{
        "debug_pause".to_string()
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()>{
        context.toggle_pause();
        Ok(())
    }
}
```

今回の例では、コマンドと結びつけましたが、システムと結びつけることも可能です。





