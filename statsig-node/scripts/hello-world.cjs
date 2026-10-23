const { Amplitude } = require('../build/index.js');

async function main() {
  const amplitude = new Amplitude();
  amplitude.init('17fcb31e58fc138462fd64bfe7add49e');

  await amplitude.track({
    eventType: 'Hello World 12:19pm',
    userId: 'server-amplitude-user-node',
    sessionId: Date.now(),
  });

  console.log('Hello World accepted by Amplitude');
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
