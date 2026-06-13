arrow keys move

-/= zoom in out

# Play pipewire pipe

```
.config/pipewire/pipewire.conf.d/pipe-tunnel.conf

context.modules = [
{   name = libpipewire-module-pipe-tunnel
    args = {
        tunnel.mode = sink
        node.name = "pipe-sink"
        node.description = "Pipe Tunnel Sink"
    }
}
]
```

connect in `qpwgraph`, then play pipe

```
mpv --demuxer=rawaudio \
    --demuxer-rawaudio-format=s16le \
    --demuxer-rawaudio-rate=48000 \
    --demuxer-rawaudio-channels=2 \
    /tmp/fifo_input
```
