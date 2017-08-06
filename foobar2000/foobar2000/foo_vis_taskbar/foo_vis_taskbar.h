#ifndef FOO_VIS_TASKBAR_H
#define FOO_VIS_TASKBAR_H

//Foobar 2000 SDK Components
#include "../SDK/foobar2000.h"
#include "../helpers/helpers.h"
#include "../../../vis_taskbar_common/vis_taskbar_common.h"

#define APPNAME								L"foo_vis_taskbar"

#define FFT_SIZE							1024
#define DATA_SIZE							1024
#define	HALF_DATA_SIZE						512
#define HALF_DATA_SIZE_MINUS_ONE			511

class foo_vis_taskbar : public vis_taskbar_common
{
private:
	static service_ptr_t<visualisation_stream>
											Stream;
	static bool								CreateStream();

	static INT_PTR CALLBACK					PreferencesPageWindowProcedure(HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam);
	static VISRGB							ShowColorDialog(HWND hOwner, VISRGB rgb);
	static SETTINGS							SettingsClone;

	bool									FillSpectrumData(PUCHAR values);

public:
	static foo_vis_taskbar					Instance;

	HWND									CreatePreferencesDialog(HWND hParent);

	void									CloneSettings();
	void									UpdateSettings();

	void									Start();
	void									Stop();
	
};

#endif //FOO_VIS_TASKBAR_H