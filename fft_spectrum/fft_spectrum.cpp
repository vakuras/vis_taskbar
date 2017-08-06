#include "fft_spectrum.h"
#include "kiss_fft_spectrum.h"

fft_spectrum* fft_spectrum::Create(int signalSize, FFTWindowType windowType, FFTImplementation implementation)
{
	fft_spectrum* fft = new kiss_fft_spectrum();
	fft->Setup(signalSize, windowType);
	return fft;
}

void fft_spectrum::Setup(int signalSize, FFTWindowType windowType) 
{
	m_SignalSize = signalSize;
	m_BinSize = (signalSize / 2) + 1;

	m_SignalNormalized = true;
	m_pSignal = new float[signalSize];

	m_CartesianUpdated = true;
	m_CartesianNormalized = true;
	m_pReal = new float[m_BinSize];
	m_pImaginary = new float[m_BinSize];

	m_PolarUpdated = true;
	m_PolarNormalized = true;
	m_pAmplitude = new float[m_BinSize];
	m_pPhase = new float[m_BinSize];

	Clear();

	m_pWindow = new float[signalSize];
	m_pInverseWindow = new float[signalSize];
	SetWindowType(windowType);
}

int fft_spectrum::GetBinSize()
{
	return m_BinSize;
}

int fft_spectrum::GetSignalSize() 
{
	return m_SignalSize;
}

void fft_spectrum::SetWindowType(FFTWindowType windowType)
{
	int half = m_SignalSize / 2;
	m_WindowType = windowType;

	switch(m_WindowType)
	{
	case FFT_WINDOW_RECTANGULAR:
		for(int i = 0; i < m_SignalSize; i++)
			m_pWindow[i] = 1;
		break;

	case FFT_WINDOW_BARTLETT:	
		for (int i = 0; i < half; i++) 
		{
			m_pWindow[i] = ((float) i / half);
			m_pWindow[i + half] = (1 - ((float) i / half));
		}
		break;

	case FFT_WINDOW_HANN:
		for(int i = 0; i < m_SignalSize; i++)
			m_pWindow[i] = .5 * (1 - cos((TWO_PI * i) / (m_SignalSize - 1)));
		break;

	case FFT_WINDOW_HAMMING:
		for(int i = 0; i < m_SignalSize; i++)
			m_pWindow[i] = .54 - .46 * cos((TWO_PI * i) / (m_SignalSize - 1));
		break;

	case FFT_WINDOW_SINE:
		for(int i = 0; i < m_SignalSize; i++)
			m_pWindow[i] = sin((PI * i) / (m_SignalSize - 1));
		break;
	}

	m_WindowSum = 0;
	for(int i = 0; i < m_SignalSize; i++)
		m_WindowSum += m_pWindow[i];

	for(int i = 0; i < m_SignalSize; i++)
		m_pInverseWindow[i] = 1. / m_pWindow[i];
}

fft_spectrum::~fft_spectrum() 
{
	delete [] m_pSignal;
	delete [] m_pReal;
	delete [] m_pImaginary;
	delete [] m_pAmplitude;
	delete [] m_pPhase;
	delete [] m_pWindow;
	delete [] m_pInverseWindow;
}

//void fft_spectrum::draw(float x, float y, float width, float height)
//{
//	ofPushStyle();
//	ofPushMatrix();
//
//	ofTranslate(x, y);
//	ofNoFill();
//	ofRect(0, 0, width, height);
//	ofTranslate(0, height);
//	ofScale(width / binSize, -height);
//	ofBeginShape();
//	getAmplitude();
//	for (int i = 0; i < binSize; i++)
//		ofVertex(i, amplitude[i]);
//	ofEndShape();
//
//	ofPopMatrix();
//	ofPopStyle();
//}

void fft_spectrum::Clear() 
{
	memset(m_pSignal, 0, sizeof(float) * m_SignalSize);
	memset(m_pReal, 0, sizeof(float) * m_BinSize);
	memset(m_pImaginary, 0, sizeof(float) * m_BinSize);
	memset(m_pAmplitude, 0, sizeof(float) * m_BinSize);
	memset(m_pPhase, 0, sizeof(float) * m_BinSize);
}

void fft_spectrum::CopySignal(float* signal, FFTSignalType signalType) 
{
	switch(signalType)
	{
	case FFT_SIGNAL_MONO:
		memcpy(m_pSignal, signal, sizeof(float) * m_SignalSize);
		break;

	case FFT_SIGNAL_LEFT:
		for(int i=0; i<m_SignalSize; i++)
			m_pSignal[i] = signal[i * 2];
		break;

	case FFT_SIGNAL_RIGHT:
		for(int i=0; i<m_SignalSize; i++)
			m_pSignal[i] = signal[i * 2 + 1];
		break;
	}
}

void fft_spectrum::CopyReal(float* real)
{
	memcpy(m_pReal, real, sizeof(float) * m_BinSize);
}

void fft_spectrum::CopyImaginary(float* imag)
{
	if(imag == NULL)
		memset(m_pImaginary, 0, sizeof(float) * m_BinSize);
	else
		memcpy(m_pImaginary, imag, sizeof(float) * m_BinSize);
}

void fft_spectrum::CopyAmplitude(float* amplitude) 
{
	memcpy(m_pAmplitude, amplitude, sizeof(float) * m_BinSize);
}

void fft_spectrum::CopyPhase(float* phase) 
{
	if(phase == NULL)
		memset(m_pPhase, 0, sizeof(float) * m_BinSize);
	else
		memcpy(m_pPhase, phase, sizeof(float) * m_BinSize);
}

void fft_spectrum::PrepareSignal()
{
	if(!m_SignalUpdated)
		UpdateSignal();

	if(!m_SignalNormalized)
		NormalizeSignal();
}

void fft_spectrum::UpdateSignal()
{
	PrepareCartesian();
	ExecuteIFFT();
	m_SignalUpdated = true;
	m_SignalNormalized = false;
}

void fft_spectrum::NormalizeSignal()
{
	float normalizer = (float) m_WindowSum / (2 * m_SignalSize);

	for (int i = 0; i < m_SignalSize; i++)
		m_pSignal[i] *= normalizer;

	m_SignalNormalized = true;
}

float* fft_spectrum::GetSignal()
{
	PrepareSignal();
	return m_pSignal;
}

void fft_spectrum::ClampSignal() 
{
	PrepareSignal();

	for(int i = 0; i < m_SignalSize; i++) 
	{
		if(m_pSignal[i] > 1)
			m_pSignal[i] = 1;

		else if(m_pSignal[i] < -1)
			m_pSignal[i] = -1;
	}
}

void fft_spectrum::PrepareCartesian()
{
	if(!m_CartesianUpdated) 
	{
		if(!m_PolarUpdated)
			ExecuteFFT();
		else
			UpdateCartesian();
	}

	if(!m_CartesianNormalized)
		NormalizeCartesian();
}

float* fft_spectrum::GetReal()
{
	PrepareCartesian();
	return m_pReal;
}

float* fft_spectrum::GetImaginary() 
{
	PrepareCartesian();
	return m_pImaginary;
}

void fft_spectrum::PreparePolar() 
{
	if(!m_PolarUpdated)
		UpdatePolar();

	if(!m_PolarNormalized)
		NormalizePolar();
}

float* fft_spectrum::GetAmplitude() 
{
	PreparePolar();
	return m_pAmplitude;
}

float* fft_spectrum::GetPhase() 
{
	PreparePolar();
	return m_pPhase;
}

void fft_spectrum::UpdateCartesian() 
{
	for(int i = 0; i < m_BinSize; i++) 
	{
		m_pReal[i] = cosf(m_pPhase[i]) * m_pAmplitude[i];
		m_pImaginary[i] = sinf(m_pPhase[i]) * m_pAmplitude[i];
	}

	m_CartesianUpdated = true;
	m_CartesianNormalized = m_PolarNormalized;
}

void fft_spectrum::NormalizeCartesian() 
{
	float normalizer = 2.0f / m_WindowSum;

	for(int i = 0; i < m_BinSize; i++) 
	{
		m_pReal[i] *= normalizer;
		m_pImaginary[i] *= normalizer;
	}

	m_CartesianNormalized = true;
}

void fft_spectrum::UpdatePolar() 
{
	PrepareCartesian();

	for(int i = 0; i < m_BinSize; i++) 
	{
		m_pAmplitude[i] = CARTESIAN_TO_AMPLITUDE(m_pReal[i], m_pImaginary[i]);
		m_pPhase[i] = CARTESIAN_TO_PHASE(m_pReal[i], m_pImaginary[i]);
	}

	m_PolarUpdated = true;
	m_PolarNormalized = m_CartesianNormalized;
}

void fft_spectrum::NormalizePolar()
{
	float normalizer = 2.0f / m_WindowSum;

	for(int i = 0; i < m_BinSize; i++)
		m_pAmplitude[i] *= normalizer;

	m_PolarNormalized = true;
}

void fft_spectrum::ClearUpdates()
{
	m_CartesianUpdated = false;
	m_PolarUpdated = false;
	m_CartesianNormalized = false;
	m_PolarNormalized = false;
	m_SignalUpdated = false;
	m_SignalNormalized = false;
}

void fft_spectrum::SetSignal(float* signal, FFTSignalType signalType)
{
	ClearUpdates();
	CopySignal(signal, signalType);
	m_SignalUpdated = true;
	m_SignalNormalized = true;
}

void fft_spectrum::SetCartesian(float* real, float* imag) 
{
	ClearUpdates();
	CopyReal(real);
	CopyImaginary(imag);
	m_CartesianUpdated = true;
	m_CartesianNormalized = true;
}

void fft_spectrum::SetPolar(float* amplitude, float* phase) 
{
	ClearUpdates();
	CopyAmplitude(amplitude);
	CopyPhase(phase);
	m_PolarUpdated = true;
	m_PolarNormalized = true;
}
