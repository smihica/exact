use super::*;
use exact_runner::{Answer, DataError, Dispatch, Outcome, Request, Store, Value};

#[derive(Default)]
struct Held {
    ready: bool,
    held: bool,
    settled: usize,
    wake: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl DataSource for Held {
    fn query(&mut self, name: &str, _: &[Value]) -> Result<Value, DataError> {
        Err(DataError::UnknownSource(name.into()))
    }

    fn answer(&mut self, _: &mut Store, _: &str, _: &[Value]) -> Result<Answer, DataError> {
        Ok(Answer::Later(Request::continuation(1)))
    }

    fn dispatch(&mut self, token: u64, _: &Store) -> Dispatch {
        assert_eq!(token, 1);
        self.held = true;
        Dispatch::Held
    }

    fn release(&mut self, _: &Store) -> Vec<(u64, Dispatch)> {
        if self.ready && self.held {
            self.held = false;
            vec![(1, Dispatch::Again)]
        } else {
            Vec::new()
        }
    }

    fn abort_wake(&mut self, wake: Arc<dyn Fn() + Send + Sync>) {
        self.wake = Some(wake);
    }

    fn parse(
        &mut self,
        _: &mut Store,
        _: &str,
        _: &[Value],
        outcome: Outcome,
    ) -> Result<Answer, DataError> {
        assert!(matches!(outcome, Outcome::Response(r) if r.status == 200));
        self.settled += 1;
        Ok(Answer::Now(Value::Bool(true)))
    }
}

#[test]
fn a_wake_without_a_reply_releases_held_work() {
    let plan = crate::test_plan(
        r#"component App
  mutation result as shape bool
  action start
    send result = work()
  view
    column width=200 height=200
      button press=start testId="start"
        text "Start"
      match result
        case some(value)
          text "Completed" testId="completed"
        case none
          text "Waiting"
"#,
    );
    let (mut p, error) = Presenter::boot_with(
        &plan,
        Held::default(),
        (200., 200.),
        1.,
        PathBuf::new(),
        PainterChoice::Cpu,
    )
    .unwrap();
    assert!(error.is_none(), "{error:?}");
    let k = p.host.kernel();
    let start = k.node_by_key(k.find_by_test_id("start")[0]).unwrap().id;
    p.tap(start).unwrap();
    assert!(p.host.runner().has_pending());
    assert!(p.parked.contains_key(&1));

    assert_eq!(p.pump(0.), None);
    assert!(p.parked.contains_key(&1), "still waiting on the source");

    // A secret lease becomes ready and wakes the host without a reply or topic.
    let source = p.host.runner_mut().data();
    source.ready = true;
    source.wake.as_ref().unwrap()();
    assert!(!p.host.has_announced());
    assert!(p.executor.drain().is_empty());
    assert_eq!(p.pump(0.), None);
    assert!(p.parked.is_empty(), "the wake releases the parked request");

    assert_eq!(p.pump(0.), None);
    assert!(!p.host.runner().has_pending());
    assert_eq!(p.host.kernel().find_by_test_id("completed").len(), 1);
    assert_eq!(p.pump(0.), None);
    assert_eq!(p.host.runner_mut().data().settled, 1);
}
