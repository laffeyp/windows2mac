def SetLastError(thread, args):
    thread.last_error = args[0]
    return None

def GetLastError(thread, args):
    return thread.last_error
