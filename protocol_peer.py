import asyncio
import json
import uuid
from nats.aio.client import Client as NATS
from datetime import datetime

SESSION_ID = "test-123"  # could be dynamic or injected
START_SUBJECT = f"plan.session.{SESSION_ID}.start"
CONTROL_SUBJECT = f"plan.session.{SESSION_ID}.control"
LOG_SUBJECT = f"plan.session.{SESSION_ID}.log"
STATE_SUBJECT = f"plan.session.{SESSION_ID}.state"
EVENTS_SUBJECT = f"plan.session.{SESSION_ID}.events"

async def main():
    nc = NATS()

    await nc.connect(servers=["nats://localhost:4222"])
    print(f"[mock_peer] Connected to NATS. Session: {SESSION_ID}")

    async def handle_start_message(msg):
        """Handle incoming start messages with manifest"""
        data = json.loads(msg.data.decode())
        print(f"[mock_peer] Received start message: {data}")
        
        manifest = data.get("manifest", [])
        dry_run = data.get("dryRun", False)
        
        print(f"[mock_peer] Processing {len(manifest)} phases (dry_run={dry_run})")
        
        # Simulate processing each phase
        for i, phase in enumerate(manifest):
            phase_id = phase.get("id", f"phase-{i}")
            
            # Send state update: running
            await nc.publish(STATE_SUBJECT, json.dumps({
                "phaseId": phase_id,
                "status": "running",
                "updated": datetime.utcnow().isoformat() + "Z"
            }).encode())
            print(f"[mock_peer] Phase {phase_id} started")
            
            # Send log message
            await nc.publish(LOG_SUBJECT, json.dumps({
                "phaseId": phase_id,
                "level": "info",
                "message": f"Executing phase: {phase.get('spec', {}).get('description', 'No description')}",
                "timestamp": datetime.utcnow().isoformat() + "Z"
            }).encode())
            
            # Simulate work
            await asyncio.sleep(0.5)
            
            # Send state update: complete
            await nc.publish(STATE_SUBJECT, json.dumps({
                "phaseId": phase_id,
                "status": "complete",
                "updated": datetime.utcnow().isoformat() + "Z"
            }).encode())
            print(f"[mock_peer] Phase {phase_id} completed")

    async def handle_control_message(msg):
        """Handle control commands like pause/resume/cancel"""
        data = json.loads(msg.data.decode())
        command = data.get("command", "unknown")
        print(f"[mock_peer] Received control command: {command}")
        
        # Send log about control command
        await nc.publish(LOG_SUBJECT, json.dumps({
            "level": "info",
            "message": f"Control command received: {command}",
            "timestamp": datetime.utcnow().isoformat() + "Z"
        }).encode())

    # Subscribe to start and control messages
    await nc.subscribe(START_SUBJECT, cb=handle_start_message)
    await nc.subscribe(CONTROL_SUBJECT, cb=handle_control_message)

    print(f"[mock_peer] Listening on {START_SUBJECT} and {CONTROL_SUBJECT}...")
    try:
        while True:
            await asyncio.sleep(1)
    except KeyboardInterrupt:
        print("[mock_peer] Exiting...")
        await nc.drain()

if __name__ == "__main__":
    asyncio.run(main())
