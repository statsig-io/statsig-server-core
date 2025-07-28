const { useState, useEffect, useMemo, useRef } = React;
const JsonView = reactJsonView.default;

function useDebouncedEffect(value, callback, delay) {
  const timeoutRef = useRef();

  useEffect(() => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);

    timeoutRef.current = setTimeout(() => {
      callback(value);
    }, delay);

    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, [value, callback, delay]);
}

function IconButton({ iconClass, onClick, label }) {
  return (
    <button
      className="icon-button"
      onClick={onClick}
      aria-label={label}
      title={label}
    >
      <i className={iconClass}></i>
    </button>
  );
}

function Toggle({ label, checked, onChange }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      <h4>{label}</h4>
      <input
        type="checkbox"
        checked={checked}
        onChange={(_) => onChange(!checked)}
      />
    </div>
  );
}

function Slider({ label, value, onChange }) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'flex-end',
        gap: 8,
        flexDirection: 'column',
      }}
    >
      <h4>{label}</h4>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <input
          type="range"
          step={1000}
          min="0"
          max="200000"
          width="100%"
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
        <span style={{ fontSize: 10 }}>{value} / 200000</span>
      </div>
    </div>
  );
}

function DockerStat({ stat }) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        width: '100%',
        alignItems: 'flex-end',
        marginBottom: 16,
        fontSize: 12,
      }}
    >
      <h3>{stat.Name.toUpperCase()}</h3>
      <p>{stat.MemUsage}</p>
      <p>{stat.CPUPerc} CPU</p>
    </div>
  );
}

function StatsSection() {
  const [dockerStats, setDockerStats] = useState({});

  useEffect(() => {
    const interval = setInterval(() => {
      fetch('/stats', { method: 'GET' })
        .then((res) => res.json())
        .then(setDockerStats);
    }, 1000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        width: '100%',
        marginTop: 16,
        alignItems: 'flex-end',
      }}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {dockerStats?.stats?.map((stat) => (
          <DockerStat key={stat.Name} stat={stat} />
        ))}
      </div>
    </div>
  );
}

function Dashboard() {
  const [state, setState] = useState({});
  const [originalState, setOriginalState] = useState({});
  const [version, setVersion] = useState(0);

  useEffect(() => {
    fetch('/state', { method: 'POST' })
      .then((res) => res.json())
      .then((newState) => {
        setState(newState);
        setOriginalState(newState);
      });
  }, [version]);

  const isDirty = useMemo(() => {
    return JSON.stringify(state) !== JSON.stringify(originalState);
  }, [state, originalState]);

  const saveStateToServer = () => {
    fetch('/state', {
      method: 'POST',
      body: JSON.stringify(state),
      headers: { 'Content-Type': 'application/json' },
    })
      .then((res) => res.json())
      .then((newState) => {
        setState(newState);
        setOriginalState(newState);
      })
      .catch(console.error);
  };

  // useDebouncedEffect(
  //   state,
  //   (newState) => {
  //     fetch('/state', {
  //       method: 'POST',
  //       body: JSON.stringify(newState),
  //       headers: { 'Content-Type': 'application/json' },
  //     })
  //       .then((res) => res.json())
  //       .then(setState)
  //       .catch(console.error);
  //   },
  //   3000,
  // );

  if (Object.keys(state).length === 0) {
    return <div>Loading...</div>;
  }

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'row',
      }}
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          width: 250,
          alignItems: 'flex-end',
          padding: 16,
          gap: 8,
        }}
      >
        <Toggle
          label={`Chaos Agent Enabled`}
          checked={state?.chaosAgent?.active === true}
          onChange={(e) => {
            const newState = JSON.parse(JSON.stringify(state));
            newState.chaosAgent.active = e;
            setState(newState);
          }}
        />
        <Toggle
          label={`Log Event Enabled (${state?.scrapi?.logEvent?.response?.status})`}
          checked={state?.scrapi?.logEvent?.response?.status === 201}
          onChange={(e) => {
            const newState = JSON.parse(JSON.stringify(state));
            newState.scrapi.logEvent.response.status = e ? 201 : 500;
            setState(newState);
          }}
        />
        <Toggle
          label="DCS Syncing Enabled"
          checked={state?.scrapi?.dcs?.syncing?.enabled === true}
          onChange={(e) => {
            const newState = JSON.parse(JSON.stringify(state));
            newState.scrapi.dcs.syncing.enabled = e;
            setState(newState);
          }}
        />
        <Toggle
          label={`DCS Enabled (${state?.scrapi?.dcs?.response?.status})`}
          checked={state?.scrapi?.dcs?.response?.status === 200}
          onChange={(e) => {
            const newState = JSON.parse(JSON.stringify(state));
            newState.scrapi.dcs.response.status = e ? 200 : 500;
            setState(newState);
          }}
        />
        <Slider
          label="Log Event QPS"
          value={state?.sdk?.logEvent?.qps ?? 0}
          onChange={(e) => {
            const newState = JSON.parse(JSON.stringify(state));
            newState.sdk.logEvent.qps = parseInt(e);
            setState(newState);
          }}
        />

        <Slider
          label="Check Gate QPS"
          value={state?.sdk?.gate?.qps ?? 0}
          onChange={(e) => {
            const newState = JSON.parse(JSON.stringify(state));
            newState.sdk.gate.qps = parseInt(e);
            setState(newState);
          }}
        />

        <StatsSection />
      </div>
      <div
        style={{
          display: 'flex',
          overflow: 'auto',
          margin: 16,
          flex: 1,
          position: 'relative',
        }}
      >
        <JsonView
          src={state}
          name={false}
          theme="tomorrow"
          style={{
            padding: 16,
          }}
          onEdit={(e) => setState(e.updated_src)}
          onDelete={(e) => setState(e.updated_src)}
          onAdd={(e) => setState(e.updated_src)}
          collapseStringsAfterLength={1000}
        />

        <div
          style={{
            padding: 16,
            position: 'absolute',
            gap: 16,
            top: 0,
            right: 0,
            left: 0,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'flex-end',
          }}
        >
          {isDirty && (
            <>
              <IconButton
                iconClass="fa-regular fa-floppy-disk"
                label="Save"
                onClick={() => saveStateToServer()}
              />

              <IconButton
                iconClass="fa-regular fa-trash-can"
                label="Discard"
                onClick={() =>
                  setState(JSON.parse(JSON.stringify(originalState)))
                }
              />
            </>
          )}

          <IconButton
            iconClass="fa-solid fa-arrows-rotate"
            label="Refresh"
            onClick={() => setVersion(version + 1)}
          />
        </div>
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<Dashboard />);
