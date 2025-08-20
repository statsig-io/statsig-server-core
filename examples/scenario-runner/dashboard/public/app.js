const { useState, useEffect, useMemo, useRef } = React;
const JsonView = reactJsonView.default;

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

function Slider({ label, value, onChange, min = 0, max = 200000 }) {
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
          min={min}
          max={max}
          width="100%"
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
        <span style={{ fontSize: 10 }}>
          <div>{value}</div> / {max}
        </span>
      </div>
    </div>
  );
}

function ServiceStat({ dockerStat, perfStats, disabledStats }) {
  const perfKey = useMemo(() => {
    return dockerStat.Name.replace('sr-', '').toLowerCase();
  }, [dockerStat.Name]);

  const perfLines = useMemo(() => {
    const stats = perfStats[perfKey] ?? [];

    return stats.map((stat) => {
      const p99 = stat.p99?.toFixed(4);
      const max = stat.max?.toFixed(4);
      const extra = stat.extra && stat.extra !== '' ? stat.extra : stat.name;
      return [`${extra} ${stat.userID}`, { p99, max, name: stat.name }];
    });
  }, [perfKey, perfStats]);

  const { showCpu, showMem, showCheckGate, showLogEvent, showGcir } =
    useMemo(() => {
      return {
        showCpu: !disabledStats.includes('cpu'),
        showMem: !disabledStats.includes('mem'),
        showCheckGate: !disabledStats.includes('check_gate'),
        showLogEvent: !disabledStats.includes('log_event'),
        showGcir: !disabledStats.includes('gcir'),
      };
    }, [disabledStats]);

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
      <h3>{dockerStat.Name.toUpperCase()}</h3>
      {showMem && <p>{dockerStat.MemUsage}</p>}
      {showCpu && <p>{dockerStat.CPUPerc} CPU</p>}
      {perfLines.map(([key, values]) => {
        if (!showCheckGate && values.name === 'check_gate') {
          return null;
        }

        if (!showLogEvent && values.name === 'log_event') {
          return null;
        }

        if (!showGcir && values.name === 'gcir') {
          return null;
        }

        return (
          <div
            key={key}
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'flex-end',
            }}
          >
            <p>{key}</p>
            <p>p99: {values.p99 ? `${values.p99}ms` : 'N/A'}</p>
            <p>max: {values.max ? `${values.max}ms` : 'N/A'}</p>
          </div>
        );
      })}
    </div>
  );
}

function StatsSelectorButton({ option, disabledOptions, setDisabledOptions }) {
  const style = {
    padding: 8,
  };

  const disabledStyle = {
    ...style,
    opacity: 0.5,
  };

  const toggle = (option) => {
    setDisabledOptions((old) => {
      if (option === 'all') {
        return [];
      }

      if (old.includes(option)) {
        return old.filter((o) => o !== option);
      }

      return [...old, option];
    });
  };

  return (
    <button
      style={disabledOptions.includes(option) ? disabledStyle : style}
      onClick={() => toggle(option)}
    >
      {option}
    </button>
  );
}

function StatsSelector({ disabledOptions, setDisabledOptions }) {
  return (
    <div>
      <StatsSelectorButton
        option="all"
        disabledOptions={disabledOptions}
        setDisabledOptions={setDisabledOptions}
      />
      <StatsSelectorButton
        option="cpu"
        disabledOptions={disabledOptions}
        setDisabledOptions={setDisabledOptions}
      />
      <StatsSelectorButton
        option="mem"
        disabledOptions={disabledOptions}
        setDisabledOptions={setDisabledOptions}
      />
      <StatsSelectorButton
        option="check_gate"
        disabledOptions={disabledOptions}
        setDisabledOptions={setDisabledOptions}
      />
      <StatsSelectorButton
        option="log_event"
        disabledOptions={disabledOptions}
        setDisabledOptions={setDisabledOptions}
      />
      <StatsSelectorButton
        option="gcir"
        disabledOptions={disabledOptions}
        setDisabledOptions={setDisabledOptions}
      />
    </div>
  );
}

function StatsSection({ disabledStats }) {
  const [dockerStats, setDockerStats] = useState([]);
  const [perfStats, setPerfStats] = useState({});

  useEffect(() => {
    const interval = setInterval(() => {
      fetch('/stats', { method: 'GET' })
        .then((res) => res.json())
        .then((res) => {
          const sortedStats = res.dockerStats.stats.sort((a, b) => {
            return a.Name.localeCompare(b.Name);
          });
          setDockerStats(sortedStats);
          setPerfStats(res.perfStats);
        });
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
        {dockerStats?.map((dockerStat) => {
          return (
            <ServiceStat
              key={dockerStat.Name}
              dockerStat={dockerStat}
              perfStats={perfStats}
              disabledStats={disabledStats}
            />
          );
        })}
      </div>
    </div>
  );
}

function Dashboard() {
  const [state, setState] = useState({});
  const [originalState, setOriginalState] = useState({});
  const [version, setVersion] = useState(0);
  const [disabledStats, setDisabledStats] = useState([
    'log_event',
    'check_gate',
    'gcir',
  ]);

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

  if (Object.keys(state).length === 0) {
    return <div>Loading...</div>;
  }

  return (
    <div>
      <div
        style={{
          display: 'flex',
          flexDirection: 'row',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: 16,
        }}
      >
        <h1>Scenario Runner</h1>
        <StatsSelector
          disabledOptions={disabledStats}
          setDisabledOptions={setDisabledStats}
        />
      </div>
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

          <Slider
            label="GCIR QPS"
            value={state?.sdk?.gcir?.qps ?? 0}
            max={10_000}
            onChange={(e) => {
              const newState = JSON.parse(JSON.stringify(state));
              newState.sdk.gcir.qps = parseInt(e);
              setState(newState);
            }}
          />

          <StatsSection disabledStats={disabledStats} />

          <div>
            <button
              style={{ padding: 8, cursor: 'pointer' }}
              onClick={() => {
                window.open(
                  'http://localhost:8000/v1/download_config_specs',
                  '_blank',
                );
              }}
            >
              View DCS v1
            </button>
            <button
              style={{ padding: 8, cursor: 'pointer' }}
              onClick={() => {
                window.open(
                  'http://localhost:8000/v2/download_config_specs',
                  '_blank',
                );
              }}
            >
              View DCS v2
            </button>
          </div>
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
              width: '100%',
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
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<Dashboard />);
