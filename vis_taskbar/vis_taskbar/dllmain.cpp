#include "vis_taskbar.h"

BOOL APIENTRY DllMain( HMODULE hModule,
                       DWORD  ul_reason_for_call,
                       LPVOID lpReserved
					 )
{
	switch (ul_reason_for_call)
	{
	case DLL_PROCESS_ATTACH:
		vis_taskbar::SetAppName(APPNAME);
		vis_taskbar::InitTrace();
		break;
	case DLL_PROCESS_DETACH:
		vis_taskbar::DeInitTrace();
		break;

	case DLL_THREAD_ATTACH:
	case DLL_THREAD_DETACH:
		break;
	}
	return TRUE;
}

