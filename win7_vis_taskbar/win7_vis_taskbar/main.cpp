#include "win7_vis_taskbar.h"

int WINAPI wWinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPWSTR lpCmdLine, int nCmdShow)
{
	win7_vis_taskbar instance;

	if (!instance.BuildUI(hInstance))
		return EXIT_FAILURE;

	if (!instance.Initialize())
		return EXIT_FAILURE;

	instance.MessageLoop();

	return EXIT_SUCCESS;
}