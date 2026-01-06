use std::fmt;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
pub enum FsEventKind {
    // Channel Events
    CHANNEL_CREATE,
    CHANNEL_STATE,
    CHANNEL_DESTROY,
    CHANNEL_ANSWER,
    CHANNEL_HANGUP,
    CHANNEL_HANGUP_COMPLETE,
    CHANNEL_PROGRESS,
    CHANNEL_PROGRESS_MEDIA,
    CHANNEL_PARK,
    CHANNEL_UNPARK,
    CHANNEL_ORIGINATE,
    CHANNEL_OUTGOING,
    CHANNEL_BRIDGE,
    CHANNEL_UNBRIDGE,
    CHANNEL_HOLD,
    CHANNEL_UNHOLD,
    CHANNEL_EXECUTE,
    CHANNEL_EXECUTE_COMPLETE,
    CHANNEL_APPLICATION,
    CHANNEL_DATA,
    CHANNEL_UUID,
    CHANNEL_CALLSTATE,
    
    // System Events
    SHUTDOWN,
    STARTUP,
    RELOAD,
    RELOADXML,
    HEARTBEAT,
    MODULE_LOAD,
    MODULE_UNLOAD,
    
    // Call Detail Records
    CDR,
    
    // Custom Events
    CUSTOM,
    
    // API Events
    API,
    COMMAND,
    
    // Background Jobs
    BACKGROUND_JOB,
}

impl fmt::Display for FsEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FsEventKind::CHANNEL_CREATE => "CHANNEL_CREATE",
            FsEventKind::CHANNEL_STATE => "CHANNEL_STATE",
            FsEventKind::CHANNEL_DESTROY => "CHANNEL_DESTROY",
            FsEventKind::CHANNEL_ANSWER => "CHANNEL_ANSWER",
            FsEventKind::CHANNEL_HANGUP => "CHANNEL_HANGUP",
            FsEventKind::CHANNEL_HANGUP_COMPLETE => "CHANNEL_HANGUP_COMPLETE",
            FsEventKind::CHANNEL_PROGRESS => "CHANNEL_PROGRESS",
            FsEventKind::CHANNEL_PROGRESS_MEDIA => "CHANNEL_PROGRESS_MEDIA",
            FsEventKind::CHANNEL_PARK => "CHANNEL_PARK",
            FsEventKind::CHANNEL_UNPARK => "CHANNEL_UNPARK",
            FsEventKind::CHANNEL_ORIGINATE => "CHANNEL_ORIGINATE",
            FsEventKind::CHANNEL_OUTGOING => "CHANNEL_OUTGOING",
            FsEventKind::CHANNEL_BRIDGE => "CHANNEL_BRIDGE",
            FsEventKind::CHANNEL_UNBRIDGE => "CHANNEL_UNBRIDGE",
            FsEventKind::CHANNEL_HOLD => "CHANNEL_HOLD",
            FsEventKind::CHANNEL_UNHOLD => "CHANNEL_UNHOLD",
            FsEventKind::CHANNEL_EXECUTE => "CHANNEL_EXECUTE",
            FsEventKind::CHANNEL_EXECUTE_COMPLETE => "CHANNEL_EXECUTE_COMPLETE",
            FsEventKind::CHANNEL_APPLICATION => "CHANNEL_APPLICATION",
            FsEventKind::CHANNEL_DATA => "CHANNEL_DATA",
            FsEventKind::CHANNEL_UUID => "CHANNEL_UUID",
            FsEventKind::CHANNEL_CALLSTATE => "CHANNEL_CALLSTATE",
            FsEventKind::SHUTDOWN => "SHUTDOWN",
            FsEventKind::STARTUP => "STARTUP",
            FsEventKind::RELOAD => "RELOAD",
            FsEventKind::RELOADXML => "RELOADXML",
            FsEventKind::HEARTBEAT => "HEARTBEAT",
            FsEventKind::MODULE_LOAD => "MODULE_LOAD",
            FsEventKind::MODULE_UNLOAD => "MODULE_UNLOAD",
            FsEventKind::CDR => "CDR",
            FsEventKind::CUSTOM => "CUSTOM",
            FsEventKind::API => "API",
            FsEventKind::COMMAND => "COMMAND",
            FsEventKind::BACKGROUND_JOB => "BACKGROUND_JOB",
        };
        write!(f, "{}", s)
    }
}

