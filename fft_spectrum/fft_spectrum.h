#pragma once

#include <Windows.h>
#define _USE_MATH_DEFINES
#include <math.h>

enum FFTWindowType 
{
	FFT_WINDOW_RECTANGULAR,
	FFT_WINDOW_BARTLETT,
	FFT_WINDOW_HANN,
	FFT_WINDOW_HAMMING,
	FFT_WINDOW_SINE
};

enum FFTImplementation 
{
	KISS_FFT
};

enum FFTSignalType
{
	FFT_SIGNAL_MONO,
	FFT_SIGNAL_LEFT,
	FFT_SIGNAL_RIGHT
};

#define PI									M_PI
#define TWO_PI								6.28318530717958647693

#define CARTESIAN_TO_AMPLITUDE(x, y)		sqrtf(x * x + y * y)
#define CARTESIAN_TO_PHASE(x,y)				atan2f(y, x)

class fft_spectrum 
{
public:
	static fft_spectrum*					Create(int signalSize = 512, FFTWindowType windowType = FFT_WINDOW_HAMMING,	FFTImplementation implementation = KISS_FFT);
	virtual ~fft_spectrum();

	void									SetSignal(float* signal, FFTSignalType signalType = FFT_SIGNAL_MONO);
	void									SetCartesian(float* real, float* imag = NULL);
	void									SetPolar(float* amplitude, float* phase = NULL);

	int										GetSignalSize();
	float*									GetSignal();
	void									ClampSignal();

	int										GetBinSize();
	float*									GetReal();
	float*									GetImaginary();
	float*									GetAmplitude();
	float*									GetPhase();

protected:
	virtual void							Setup(int signalSize, FFTWindowType windowType);
	virtual void							ExecuteFFT() = 0;
	virtual void							ExecuteIFFT() = 0;

	void									Clear();

	void									SetWindowType(FFTWindowType windowType);

	inline void								RunWindow(float* signal)
	{
		if(m_WindowType != FFT_WINDOW_RECTANGULAR)
			for(int i = 0; i < m_SignalSize; i++)
				signal[i] *= m_pWindow[i];
	}

	inline void								RunInverseWindow(float* signal)
	{
		if(m_WindowType != FFT_WINDOW_RECTANGULAR)
			for(int i = 0; i < m_SignalSize; i++)
				signal[i] *= m_pInverseWindow[i];
	}

	void									PrepareSignal();
	void									UpdateSignal();
	void									NormalizeSignal();
	void									CopySignal(float* signal, FFTSignalType signalType);

	void									PrepareCartesian();
	void									UpdateCartesian();
	void									NormalizeCartesian();
	void									CopyReal(float* real);
	void									CopyImaginary(float* imag);

	void									PreparePolar();
	void									UpdatePolar();
	void									NormalizePolar();
	void									CopyAmplitude(float* amplitude);
	void									CopyPhase(float* phase);

	void									ClearUpdates();

	FFTWindowType							m_WindowType;
	float									m_WindowSum;
	float*									m_pWindow;
	float*									m_pInverseWindow;

	float*									m_pSignal;
	bool									m_SignalUpdated;
	bool									m_SignalNormalized;

	int										m_SignalSize;
	int										m_BinSize;

	float*									m_pReal;
	float*									m_pImaginary;
	bool									m_CartesianUpdated;
	bool									m_CartesianNormalized;
	float*									m_pAmplitude;
	float*									m_pPhase;
	bool									m_PolarUpdated;
	bool									m_PolarNormalized;
};
