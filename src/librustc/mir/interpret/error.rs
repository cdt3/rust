use std::{fmt, env};

use mir;
use middle::const_val::ConstEvalErr;
use ty::{FnSig, Ty, layout};
use ty::layout::{Size, Align};

use super::{
    Pointer, Lock, AccessKind
};

use backtrace::Backtrace;

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
pub struct EvalError<'tcx> {
    pub kind: EvalErrorKind<'tcx, u64>,
}

impl<'tcx> From<EvalErrorKind<'tcx, u64>> for EvalError<'tcx> {
    fn from(kind: EvalErrorKind<'tcx, u64>) -> Self {
        match env::var("MIRI_BACKTRACE") {
            Ok(ref val) if !val.is_empty() => {
                let backtrace = Backtrace::new();

                use std::fmt::Write;
                let mut trace_text = "\n\nAn error occurred in miri:\n".to_string();
                write!(trace_text, "backtrace frames: {}\n", backtrace.frames().len()).unwrap();
                'frames: for (i, frame) in backtrace.frames().iter().enumerate() {
                    if frame.symbols().is_empty() {
                        write!(trace_text, "{}: no symbols\n", i).unwrap();
                    }
                    for symbol in frame.symbols() {
                        write!(trace_text, "{}: ", i).unwrap();
                        if let Some(name) = symbol.name() {
                            write!(trace_text, "{}\n", name).unwrap();
                        } else {
                            write!(trace_text, "<unknown>\n").unwrap();
                        }
                        write!(trace_text, "\tat ").unwrap();
                        if let Some(file_path) = symbol.filename() {
                            write!(trace_text, "{}", file_path.display()).unwrap();
                        } else {
                            write!(trace_text, "<unknown_file>").unwrap();
                        }
                        if let Some(line) = symbol.lineno() {
                            write!(trace_text, ":{}\n", line).unwrap();
                        } else {
                            write!(trace_text, "\n").unwrap();
                        }
                    }
                }
                error!("{}", trace_text);
            },
            _ => {},
        }
        EvalError {
            kind,
        }
    }
}

pub type AssertMessage<'tcx> = EvalErrorKind<'tcx, mir::Operand<'tcx>>;

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum EvalErrorKind<'tcx, O> {
    /// This variant is used by machines to signal their own errors that do not
    /// match an existing variant
    MachineError(String),
    FunctionPointerTyMismatch(FnSig<'tcx>, FnSig<'tcx>),
    NoMirFor(String),
    UnterminatedCString(Pointer),
    DanglingPointerDeref,
    DoubleFree,
    InvalidMemoryAccess,
    InvalidFunctionPointer,
    InvalidBool,
    InvalidDiscriminant,
    PointerOutOfBounds {
        ptr: Pointer,
        access: bool,
        allocation_size: Size,
    },
    InvalidNullPointerUsage,
    ReadPointerAsBytes,
    ReadBytesAsPointer,
    InvalidPointerMath,
    ReadUndefBytes,
    DeadLocal,
    InvalidBoolOp(mir::BinOp),
    Unimplemented(String),
    DerefFunctionPointer,
    ExecuteMemory,
    BoundsCheck { len: O, index: O },
    Overflow(mir::BinOp),
    OverflowNeg,
    DivisionByZero,
    RemainderByZero,
    Intrinsic(String),
    InvalidChar(u128),
    StackFrameLimitReached,
    OutOfTls,
    TlsOutOfBounds,
    AbiViolation(String),
    AlignmentCheckFailed {
        required: Align,
        has: Align,
    },
    MemoryLockViolation {
        ptr: Pointer,
        len: u64,
        frame: usize,
        access: AccessKind,
        lock: Lock,
    },
    MemoryAcquireConflict {
        ptr: Pointer,
        len: u64,
        kind: AccessKind,
        lock: Lock,
    },
    InvalidMemoryLockRelease {
        ptr: Pointer,
        len: u64,
        frame: usize,
        lock: Lock,
    },
    DeallocatedLockedMemory {
        ptr: Pointer,
        lock: Lock,
    },
    ValidationFailure(String),
    CalledClosureAsFunction,
    VtableForArgumentlessMethod,
    ModifiedConstantMemory,
    AssumptionNotHeld,
    InlineAsm,
    TypeNotPrimitive(Ty<'tcx>),
    ReallocatedWrongMemoryKind(String, String),
    DeallocatedWrongMemoryKind(String, String),
    ReallocateNonBasePtr,
    DeallocateNonBasePtr,
    IncorrectAllocationInformation(Size, Size, Align, Align),
    Layout(layout::LayoutError<'tcx>),
    HeapAllocZeroBytes,
    HeapAllocNonPowerOfTwoAlignment(u64),
    Unreachable,
    Panic,
    ReadFromReturnPointer,
    PathNotFound(Vec<String>),
    UnimplementedTraitSelection,
    /// Abort in case type errors are reached
    TypeckError,
    /// Cannot compute this constant because it depends on another one
    /// which already produced an error
    ReferencedConstant(ConstEvalErr<'tcx>),
    GeneratorResumedAfterReturn,
    GeneratorResumedAfterPanic,
}

pub type EvalResult<'tcx, T = ()> = Result<T, EvalError<'tcx>>;

impl<'tcx, O> EvalErrorKind<'tcx, O> {
    pub fn description(&self) -> &str {
        use self::EvalErrorKind::*;
        match *self {
            MachineError(ref inner) => inner,
            FunctionPointerTyMismatch(..) =>
                "tried to call a function through a function pointer of a different type",
            InvalidMemoryAccess =>
                "tried to access memory through an invalid pointer",
            DanglingPointerDeref =>
                "dangling pointer was dereferenced",
            DoubleFree =>
                "tried to deallocate dangling pointer",
            InvalidFunctionPointer =>
                "tried to use a function pointer after offsetting it",
            InvalidBool =>
                "invalid boolean value read",
            InvalidDiscriminant =>
                "invalid enum discriminant value read",
            PointerOutOfBounds { .. } =>
                "pointer offset outside bounds of allocation",
            InvalidNullPointerUsage =>
                "invalid use of NULL pointer",
            MemoryLockViolation { .. } =>
                "memory access conflicts with lock",
            MemoryAcquireConflict { .. } =>
                "new memory lock conflicts with existing lock",
            ValidationFailure(..) =>
                "type validation failed",
            InvalidMemoryLockRelease { .. } =>
                "invalid attempt to release write lock",
            DeallocatedLockedMemory { .. } =>
                "tried to deallocate memory in conflict with a lock",
            ReadPointerAsBytes =>
                "a raw memory access tried to access part of a pointer value as raw bytes",
            ReadBytesAsPointer =>
                "a memory access tried to interpret some bytes as a pointer",
            InvalidPointerMath =>
                "attempted to do invalid arithmetic on pointers that would leak base addresses, e.g. comparing pointers into different allocations",
            ReadUndefBytes =>
                "attempted to read undefined bytes",
            DeadLocal =>
                "tried to access a dead local variable",
            InvalidBoolOp(_) =>
                "invalid boolean operation",
            Unimplemented(ref msg) => msg,
            DerefFunctionPointer =>
                "tried to dereference a function pointer",
            ExecuteMemory =>
                "tried to treat a memory pointer as a function pointer",
            BoundsCheck{..} =>
                "array index out of bounds",
            Intrinsic(..) =>
                "intrinsic failed",
            NoMirFor(..) =>
                "mir not found",
            InvalidChar(..) =>
                "tried to interpret an invalid 32-bit value as a char",
            StackFrameLimitReached =>
                "reached the configured maximum number of stack frames",
            OutOfTls =>
                "reached the maximum number of representable TLS keys",
            TlsOutOfBounds =>
                "accessed an invalid (unallocated) TLS key",
            AbiViolation(ref msg) => msg,
            AlignmentCheckFailed{..} =>
                "tried to execute a misaligned read or write",
            CalledClosureAsFunction =>
                "tried to call a closure through a function pointer",
            VtableForArgumentlessMethod =>
                "tried to call a vtable function without arguments",
            ModifiedConstantMemory =>
                "tried to modify constant memory",
            AssumptionNotHeld =>
                "`assume` argument was false",
            InlineAsm =>
                "miri does not support inline assembly",
            TypeNotPrimitive(_) =>
                "expected primitive type, got nonprimitive",
            ReallocatedWrongMemoryKind(_, _) =>
                "tried to reallocate memory from one kind to another",
            DeallocatedWrongMemoryKind(_, _) =>
                "tried to deallocate memory of the wrong kind",
            ReallocateNonBasePtr =>
                "tried to reallocate with a pointer not to the beginning of an existing object",
            DeallocateNonBasePtr =>
                "tried to deallocate with a pointer not to the beginning of an existing object",
            IncorrectAllocationInformation(..) =>
                "tried to deallocate or reallocate using incorrect alignment or size",
            Layout(_) =>
                "rustc layout computation failed",
            UnterminatedCString(_) =>
                "attempted to get length of a null terminated string, but no null found before end of allocation",
            HeapAllocZeroBytes =>
                "tried to re-, de- or allocate zero bytes on the heap",
            HeapAllocNonPowerOfTwoAlignment(_) =>
                "tried to re-, de-, or allocate heap memory with alignment that is not a power of two",
            Unreachable =>
                "entered unreachable code",
            Panic =>
                "the evaluated program panicked",
            ReadFromReturnPointer =>
                "tried to read from the return pointer",
            PathNotFound(_) =>
                "a path could not be resolved, maybe the crate is not loaded",
            UnimplementedTraitSelection =>
                "there were unresolved type arguments during trait selection",
            TypeckError =>
                "encountered constants with type errors, stopping evaluation",
            ReferencedConstant(_) =>
                "referenced constant has errors",
            Overflow(mir::BinOp::Add) => "attempt to add with overflow",
            Overflow(mir::BinOp::Sub) => "attempt to subtract with overflow",
            Overflow(mir::BinOp::Mul) => "attempt to multiply with overflow",
            Overflow(mir::BinOp::Div) => "attempt to divide with overflow",
            Overflow(mir::BinOp::Rem) => "attempt to calculate the remainder with overflow",
            OverflowNeg => "attempt to negate with overflow",
            Overflow(mir::BinOp::Shr) => "attempt to shift right with overflow",
            Overflow(mir::BinOp::Shl) => "attempt to shift left with overflow",
            Overflow(op) => bug!("{:?} cannot overflow", op),
            DivisionByZero => "attempt to divide by zero",
            RemainderByZero => "attempt to calculate the remainder with a divisor of zero",
            GeneratorResumedAfterReturn => "generator resumed after completion",
            GeneratorResumedAfterPanic => "generator resumed after panicking",
        }
    }
}

impl<'tcx> fmt::Display for EvalError<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}

impl<'tcx, O: fmt::Debug> fmt::Debug for EvalErrorKind<'tcx, O> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::EvalErrorKind::*;
        match *self {
            PointerOutOfBounds { ptr, access, allocation_size } => {
                write!(f, "{} at offset {}, outside bounds of allocation {} which has size {}",
                       if access { "memory access" } else { "pointer computed" },
                       ptr.offset.bytes(), ptr.alloc_id, allocation_size.bytes())
            },
            MemoryLockViolation { ptr, len, frame, access, ref lock } => {
                write!(f, "{:?} access by frame {} at {:?}, size {}, is in conflict with lock {:?}",
                       access, frame, ptr, len, lock)
            }
            MemoryAcquireConflict { ptr, len, kind, ref lock } => {
                write!(f, "new {:?} lock at {:?}, size {}, is in conflict with lock {:?}",
                       kind, ptr, len, lock)
            }
            InvalidMemoryLockRelease { ptr, len, frame, ref lock } => {
                write!(f, "frame {} tried to release memory write lock at {:?}, size {}, but cannot release lock {:?}",
                       frame, ptr, len, lock)
            }
            DeallocatedLockedMemory { ptr, ref lock } => {
                write!(f, "tried to deallocate memory at {:?} in conflict with lock {:?}",
                       ptr, lock)
            }
            ValidationFailure(ref err) => {
                write!(f, "type validation failed: {}", err)
            }
            NoMirFor(ref func) => write!(f, "no mir for `{}`", func),
            FunctionPointerTyMismatch(sig, got) =>
                write!(f, "tried to call a function with sig {} through a function pointer of type {}", sig, got),
            BoundsCheck { ref len, ref index } =>
                write!(f, "index out of bounds: the len is {:?} but the index is {:?}", len, index),
            ReallocatedWrongMemoryKind(ref old, ref new) =>
                write!(f, "tried to reallocate memory from {} to {}", old, new),
            DeallocatedWrongMemoryKind(ref old, ref new) =>
                write!(f, "tried to deallocate {} memory but gave {} as the kind", old, new),
            Intrinsic(ref err) =>
                write!(f, "{}", err),
            InvalidChar(c) =>
                write!(f, "tried to interpret an invalid 32-bit value as a char: {}", c),
            AlignmentCheckFailed { required, has } =>
               write!(f, "tried to access memory with alignment {}, but alignment {} is required",
                      has.abi(), required.abi()),
            TypeNotPrimitive(ty) =>
                write!(f, "expected primitive type, got {}", ty),
            Layout(ref err) =>
                write!(f, "rustc layout computation failed: {:?}", err),
            PathNotFound(ref path) =>
                write!(f, "Cannot find path {:?}", path),
            MachineError(ref inner) =>
                write!(f, "{}", inner),
            IncorrectAllocationInformation(size, size2, align, align2) =>
                write!(f, "incorrect alloc info: expected size {} and align {}, got size {} and align {}", size.bytes(), align.abi(), size2.bytes(), align2.abi()),
            _ => write!(f, "{}", self.description()),
        }
    }
}
