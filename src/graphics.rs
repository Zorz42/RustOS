#[repr(C)]
struct CtrlHeader {
    ctrl_type: CtrlType,
    flags: u32,
    fence_id: u64,
    ctx_id: u32,
    padding: u32
}

#[repr(u32)]
enum CtrlType {
    /* 2d commands */
    CmdGetDisplayInfo = 0x0100,
    CmdResourceCreate2d,
    CmdResourceUref,
    CmdSetScanout,
    CmdResourceFlush,
    CmdTransferToHost2d,
    CmdResourceAttachBacking,
    CmdResourceDetachBacking,
    CmdGetCapsetInfo,
    CmdGetCapset,
    CmdGetEdid,
    /* cursor commands */
    CmdUpdateCursor = 0x0300,
    CmdMoveCursor,
    /* success responses */
    RespOkNoData = 0x1100,
    RespOkDisplayInfo,
    RespOkCapsetInfo,
    RespOkCapset,
    RespOkEdid,
    /* error responses */
    RespErrUnspec = 0x1200,
    RespErrOutOfMemory,
    RespErrInvalidScanoutId,
    RespErrInvalidResourceId,
    RespErrInvalidContextId,
    RespErrInvalidParameter,
}

#[repr(u32)]
enum Formats {
    B8G8R8A8Unorm = 1,
    B8G8R8X8Unorm = 2,
    A8R8G8B8Unorm = 3,
    X8R8G8B8Unorm = 4,
    R8G8B8A8Unorm = 67,
    X8B8G8R8Unorm = 68,
    A8B8G8R8Unorm = 121,
    R8G8B8X8Unorm = 134,
}

pub struct Device {
    queue:        *mut Queue,
    dev:          *mut u32,
    idx:          u16,
    ack_used_idx: u16,
    framebuffer:  *mut Pixel,
    width:        u32,
    height:       u32,
}

struct Request<RqT, RpT> {
    request: RqT,
    response: RpT,
}
impl<RqT, RpT> Request<RqT, RpT> {
    pub fn new(request: RqT) -> *mut Self {
        let sz = size_of::<RqT>() + size_of::<RpT>();
        let ptr = kmalloc(sz) as *mut Self;
        unsafe {
            (*ptr).request = request;
        }
        ptr
    }
}